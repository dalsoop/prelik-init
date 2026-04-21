//! pxi-disk — 호스트 LVM / swap / 파일시스템 확장.
//! root LV 확장 (lvextend+resize2fs), swap 증설 (별도 LV 추가), VG/LV 현황.

use clap::{Parser, Subcommand};
use pxi_core::common;
use std::fs;

#[derive(Parser)]
#[command(name = "pxi-disk", about = "호스트 LVM/디스크/swap 관리")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// VG / LV / 파일시스템 / swap 현황
    Info,
    /// 루트 LV 확장 (lvextend + resize2fs). 다른 LV 건드리지 않음
    ResizeRoot {
        /// 확장 크기. lvextend -L 문법 (예: +15G 증분 / 120G 절대)
        #[arg(long)]
        size: String,
        /// 대상 VG (기본: pve)
        #[arg(long, default_value = "pve")]
        vg: String,
        /// 실제 실행 없이 계획만
        #[arg(long)]
        dry_run: bool,
    },
    /// Swap 확장 — 현재 swap 유지, 부족분만 새 LV 생성 + swapon + /etc/fstab 등록
    ExpandSwap {
        /// 총 목표 swap 용량 (예: 40G)
        #[arg(long)]
        total: String,
        /// 새 swap LV를 둘 VG (기본: pve). 다른 VG에 두면 I/O 분산
        #[arg(long, default_value = "pve")]
        vg: String,
        /// 실제 실행 없이 계획만
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Info => info(),
        Cmd::ResizeRoot { size, vg, dry_run } => resize_root(&size, &vg, dry_run),
        Cmd::ExpandSwap { total, vg, dry_run } => expand_swap(&total, &vg, dry_run),
    }
}

fn info() -> anyhow::Result<()> {
    println!("=== VG ===");
    common::run("vgs", &[])?;
    println!("\n=== LV ===");
    common::run("lvs", &["-o", "vg_name,lv_name,lv_size,data_percent,pool_lv"])?;
    println!("\n=== 파일시스템 ===");
    common::run("df", &["-hT", "-x", "tmpfs", "-x", "devtmpfs"])?;
    println!("\n=== Swap ===");
    common::run("swapon", &["--show"])?;
    common::run("free", &["-h"])?;
    Ok(())
}

fn resize_root(size: &str, vg: &str, dry_run: bool) -> anyhow::Result<()> {
    let lv_path = format!("/dev/{vg}/root");
    let (is_inc, req_gib) = parse_size_spec(size)?;

    let current_gib = lv_size_gib(&lv_path)
        .map_err(|e| anyhow::anyhow!("{} 크기 조회 실패: {}\n(LV 존재 확인: lvs {})", lv_path, e, vg))?;
    let vg_free_gib = vg_free_gib(vg)?;

    let delta_gib = if is_inc {
        req_gib as f64
    } else {
        req_gib as f64 - current_gib
    };

    println!("=== Root LV 확장 점검 ===");
    println!("  LV           : {lv_path}");
    println!("  현재 크기    : {:.2} G", current_gib);
    println!("  요청         : {} ({})", size, if is_inc { "증분" } else { "절대" });
    println!("  필요 추가분  : {:.2} G", delta_gib);
    println!("  VG {vg} 여유 : {:.2} G", vg_free_gib);

    if delta_gib <= 0.0 {
        anyhow::bail!(
            "요청 크기({:.2} G)가 현재 크기({:.2} G) 이하 — 온라인 축소는 지원 안 함.",
            req_gib as f64, current_gib
        );
    }
    if delta_gib > vg_free_gib {
        anyhow::bail!(
            "VG {} 여유({:.2} G) 부족 — 필요 {:.2} G.\n  해결: (a) 물리 디스크 추가 후 vgextend, (b) 씬풀/스냅샷 정리, (c) 증분 크기 줄이기 (예: +{:.0}G)",
            vg, vg_free_gib, delta_gib, vg_free_gib.floor().max(1.0)
        );
    }

    if dry_run {
        println!("\n(dry-run) 실행 예정:");
        println!("  lvextend -L {size} {lv_path}");
        println!("  resize2fs {lv_path}");
        return Ok(());
    }

    common::run("lvextend", &["-L", size, &lv_path])?;
    common::run("resize2fs", &[&lv_path])?;

    println!("\n=== 확장 후 ===");
    common::run("df", &["-h", "/"])?;
    Ok(())
}

fn expand_swap(total: &str, vg: &str, dry_run: bool) -> anyhow::Result<()> {
    let target_gib = parse_gib(total)?;

    let current_out =
        common::run_capture("swapon", &["--show=SIZE", "--bytes", "--noheadings"])
            .unwrap_or_default();
    let current_bytes: u64 = current_out
        .lines()
        .filter_map(|l| l.trim().parse::<u64>().ok())
        .sum();
    let current_gib = current_bytes / (1024 * 1024 * 1024);

    let vg_free_gib_val = vg_free_gib(vg)?;
    let other_vgs = other_vgs_with_space(vg).unwrap_or_default();

    println!("=== Swap 확장 점검 ===");
    println!("  현재 swap    : {} G", current_gib);
    println!("  목표         : {} G", target_gib);
    println!("  VG {vg} 여유 : {:.2} G", vg_free_gib_val);

    if current_gib >= target_gib {
        println!("이미 목표 이상. 아무것도 하지 않음.");
        return Ok(());
    }
    let delta = target_gib - current_gib;
    println!("  필요 추가분  : {} G", delta);

    if (delta as f64) > vg_free_gib_val {
        let mut msg = format!(
            "VG {} 여유({:.2} G) 부족 — 필요 {} G.",
            vg, vg_free_gib_val, delta
        );
        if !other_vgs.is_empty() {
            msg.push_str("\n  다른 VG 후보:");
            for (name, free) in &other_vgs {
                if *free >= delta as f64 {
                    msg.push_str(&format!("\n    --vg {} (여유 {:.2} G) ✓", name, free));
                } else {
                    msg.push_str(&format!("\n    --vg {} (여유 {:.2} G — 여전히 부족)", name, free));
                }
            }
        } else {
            msg.push_str("\n  해결: (a) 물리 디스크 추가 후 vgextend, (b) --total 을 더 낮게");
        }
        anyhow::bail!(msg);
    }

    let lv_name = next_swap_lv_name(vg)?;
    let lv_path = format!("/dev/{vg}/{lv_name}");
    println!("  추가할 LV    : {lv_path} (+{delta}G)");

    if dry_run {
        println!("\n(dry-run) 실행 예정:");
        println!("  lvcreate -L {delta}G -n {lv_name} {vg}");
        println!("  mkswap {lv_path}");
        println!("  swapon {lv_path}");
        println!("  /etc/fstab 에 '{lv_path} none swap sw 0 0' 추가");
        return Ok(());
    }

    common::run("lvcreate", &["-L", &format!("{delta}G"), "-n", &lv_name, vg])?;
    common::run("mkswap", &[&lv_path])?;
    common::run("swapon", &[&lv_path])?;

    let fstab = fs::read_to_string("/etc/fstab").unwrap_or_default();
    if !fstab.contains(&lv_path) {
        let append = format!("{lv_path} none swap sw 0 0\n");
        let new_fstab = if fstab.is_empty() || fstab.ends_with('\n') {
            format!("{fstab}{append}")
        } else {
            format!("{fstab}\n{append}")
        };
        fs::write("/etc/fstab", new_fstab)?;
        println!("/etc/fstab 업데이트");
    }

    println!("\n=== 확장 후 ===");
    common::run("swapon", &["--show"])?;
    common::run("free", &["-h"])?;
    Ok(())
}

fn parse_gib(s: &str) -> anyhow::Result<u64> {
    let up = s.trim().to_uppercase();
    let stripped = up
        .strip_suffix("GIB")
        .or_else(|| up.strip_suffix("GB"))
        .or_else(|| up.strip_suffix('G'))
        .ok_or_else(|| anyhow::anyhow!("크기는 G/GB/GiB 단위로 지정 (예: 40G)"))?;
    Ok(stripped.trim().parse()?)
}

/// "+15G" → (true, 15), "150G" → (false, 150)
fn parse_size_spec(spec: &str) -> anyhow::Result<(bool, u64)> {
    let s = spec.trim();
    let (is_inc, rest) = match s.strip_prefix('+') {
        Some(r) => (true, r),
        None => (false, s),
    };
    Ok((is_inc, parse_gib(rest)?))
}

/// VG 여유 공간 (GiB, 소수점)
fn vg_free_gib(vg: &str) -> anyhow::Result<f64> {
    let out = common::run_capture(
        "vgs",
        &["--noheadings", "-o", "vg_free", "--units", "b", "--nosuffix", vg],
    )?;
    let bytes: u64 = out.trim().parse()
        .map_err(|_| anyhow::anyhow!("VG {} 여유 공간 조회 실패 (출력: {})", vg, out.trim()))?;
    Ok(bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// LV 크기 (GiB, 소수점)
fn lv_size_gib(lv_path: &str) -> anyhow::Result<f64> {
    let out = common::run_capture(
        "lvs",
        &["--noheadings", "-o", "lv_size", "--units", "b", "--nosuffix", lv_path],
    )?;
    let bytes: u64 = out.trim().parse()
        .map_err(|_| anyhow::anyhow!("LV {} 크기 조회 실패", lv_path))?;
    Ok(bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// 현재 VG 외에 공간이 있는 다른 VG 목록
fn other_vgs_with_space(exclude: &str) -> anyhow::Result<Vec<(String, f64)>> {
    let out = common::run_capture(
        "vgs",
        &["--noheadings", "-o", "vg_name,vg_free", "--units", "b", "--nosuffix"],
    )?;
    let mut result = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        if parts[0] == exclude { continue; }
        if let Ok(bytes) = parts[1].parse::<u64>() {
            let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            if gib > 0.5 {
                result.push((parts[0].to_string(), gib));
            }
        }
    }
    Ok(result)
}

fn next_swap_lv_name(vg: &str) -> anyhow::Result<String> {
    let out = common::run_capture("lvs", &["--noheadings", "-o", "lv_name", vg])
        .unwrap_or_default();
    let names: Vec<String> = out.lines().map(|l| l.trim().to_string()).collect();
    for i in 2u32..=99 {
        let name = format!("swap{i}");
        if !names.iter().any(|n| n == &name) {
            return Ok(name);
        }
    }
    anyhow::bail!("swap2..swap99 모두 사용 중")
}
