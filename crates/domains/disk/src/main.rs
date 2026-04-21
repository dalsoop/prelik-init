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
    /// VG / LV / 파일시스템 / swap 현황 (+ stacked LVM 자동 감지)
    Info,
    /// Stacked LVM 감사 — 다른 VG 의 LV 를 PV 로 쓰는 교차 의존성 나열
    StackedStatus,
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
    /// 타 VG 의 LV 를 PV 로 편입 (stacked LVM). 교차 VG 의존 생성 — 위험 플래그 필수
    VgExtendFromLv {
        /// 편입할 대상 VG (예: pve)
        #[arg(long)]
        target: String,
        /// 새 LV 를 만들 소스 VG (예: nvme-1tb)
        #[arg(long)]
        source: String,
        /// 새 LV (= PV) 크기 (예: 200G)
        #[arg(long)]
        size: String,
        /// 새 LV 이름 (기본: <target>-ext)
        #[arg(long)]
        name: Option<String>,
        /// **필수** — cross-VG 의존(stacked LVM) 리스크 인지함
        #[arg(long)]
        i_understand_stacked_lvm_risk: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Stacked LVM 해체 — pvmove + vgreduce + pvremove + lvremove (가능할 때만)
    Unstack {
        /// 제거할 PV 경로 (예: /dev/nvme-1tb/pve-ext). `stacked-status` 로 확인
        #[arg(long)]
        pv: String,
        /// 해당 PV 가 속한 대상 VG (기본: pve)
        #[arg(long, default_value = "pve")]
        vg: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// 씬풀 축소 절차 — 실행 안 하고 플래닝만 출력 (오프라인 전용, 백업 필수)
    PlanThinReclaim {
        /// 축소할 씬풀 (기본: pve/data)
        #[arg(long, default_value = "pve/data")]
        pool: String,
        /// 새 씬풀 크기 (예: 2.5T)
        #[arg(long)]
        new_size: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Info => info(),
        Cmd::StackedStatus => stacked_status(),
        Cmd::ResizeRoot { size, vg, dry_run } => resize_root(&size, &vg, dry_run),
        Cmd::ExpandSwap { total, vg, dry_run } => expand_swap(&total, &vg, dry_run),
        Cmd::VgExtendFromLv {
            target,
            source,
            size,
            name,
            i_understand_stacked_lvm_risk,
            dry_run,
        } => vg_extend_from_lv(
            &target,
            &source,
            &size,
            name.as_deref(),
            i_understand_stacked_lvm_risk,
            dry_run,
        ),
        Cmd::Unstack { pv, vg, dry_run } => unstack(&pv, &vg, dry_run),
        Cmd::PlanThinReclaim { pool, new_size } => plan_thin_reclaim(&pool, &new_size),
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

    // Stacked LVM 자동 경고
    let stacked = find_stacked_pvs().unwrap_or_default();
    if !stacked.is_empty() {
        println!("\n\x1b[1;33m=== ⚠ Stacked LVM 감지됨 ===\x1b[0m");
        for (pv, target_vg, source_vg) in &stacked {
            println!("  {} → VG '{}' 에 편입됨 (원래 VG '{}' 의 LV)", pv, target_vg, source_vg);
        }
        println!("  교차 VG 의존성 — 해제 권장: pxi-disk unstack --pv <위 경로>");
    }
    Ok(())
}

fn stacked_status() -> anyhow::Result<()> {
    println!("=== Stacked LVM 감사 ===");
    println!("(다른 VG 의 LV 가 PV 로 편입된 구조. 디스크 고장 시 cross-VG 전파 위험)\n");
    let stacked = find_stacked_pvs()?;
    if stacked.is_empty() {
        println!("✓ Stacked LVM 없음 — 모든 PV 가 물리 파티션");
        return Ok(());
    }
    for (pv, target_vg, source_vg) in &stacked {
        println!("  PV:        {}", pv);
        println!("  대상 VG:   {} (편입된 쪽)", target_vg);
        println!("  소스 VG:   {} (LV 원 소속)", source_vg);
        println!("  해제:      pxi-disk unstack --pv {} --vg {}", pv, target_vg);
        println!();
    }
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

// ---------------------------------------------------------------------------
// Stacked LVM 감지 / 해체 / 의도적 구성
// ---------------------------------------------------------------------------

/// 다른 VG 의 LV 를 PV 로 쓰고 있는 경우를 반환.
/// (pv_path, target_vg, source_vg)
fn find_stacked_pvs() -> anyhow::Result<Vec<(String, String, String)>> {
    // scan_lvs=1 켜져 있어야 LV-as-PV 가 pvs 목록에 뜸
    let out = common::run_capture(
        "pvs",
        &["--config", "devices/scan_lvs=1", "--noheadings",
          "-o", "pv_name,vg_name"],
    ).unwrap_or_default();

    let mut result = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let pv_path = parts[0];
        let target_vg = parts[1];
        // /dev/<source_vg>/<lv_name> 형식이면 stacked
        if let Some(rest) = pv_path.strip_prefix("/dev/") {
            let segs: Vec<&str> = rest.split('/').collect();
            if segs.len() == 2 {
                // 매핑된 VG 이름이 실존하는지 확인
                let source_vg = segs[0];
                let vg_check = common::run_capture(
                    "vgs", &["--noheadings", "-o", "vg_name", source_vg]
                ).unwrap_or_default();
                if !vg_check.trim().is_empty() {
                    result.push((pv_path.to_string(), target_vg.to_string(), source_vg.to_string()));
                }
            }
        }
    }
    Ok(result)
}

fn vg_extend_from_lv(
    target: &str,
    source: &str,
    size: &str,
    name: Option<&str>,
    confirmed: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    if !confirmed {
        anyhow::bail!(
            "{}",
            "\
stacked LVM (cross-VG 의존성) 을 만듭니다. 생성 후 대상 VG 는 소스 VG 디스크 고장 시 부분 손상.
명시적 확인 플래그 필요: --i-understand-stacked-lvm-risk

대안:
  • 물리 디스크 추가 후 vgextend (리스크 없음)
  • 씬풀 축소 (오프라인) → plan-thin-reclaim 으로 절차 확인"
        );
    }

    let lv_name = name.map(|s| s.to_string()).unwrap_or_else(|| format!("{target}-ext"));
    let lv_path = format!("/dev/{source}/{lv_name}");
    let (_is_inc, size_gib) = parse_size_spec(size)?;
    let src_free = vg_free_gib(source)?;

    println!("=== VG 확장 (from LV) 점검 ===");
    println!("  소스 VG       : {source} (여유 {:.2} G)", src_free);
    println!("  대상 VG       : {target}");
    println!("  새 LV (=PV)   : {lv_path} ({size})");
    if (size_gib as f64) > src_free {
        anyhow::bail!("소스 VG {} 여유({:.2} G) < 요청({} G)", source, src_free, size_gib);
    }

    if dry_run {
        println!("\n(dry-run) 실행 예정:");
        println!("  lvcreate --yes -L {size} -n {lv_name} {source}");
        println!("  pvcreate --config 'devices/scan_lvs=1' {lv_path}");
        println!("  vgextend {target} {lv_path}");
        println!("  (lvm.conf scan_lvs = 1 영구 적용 필요)");
        return Ok(());
    }

    common::run("lvcreate", &["--yes", "-L", size, "-n", &lv_name, source])?;
    common::run("pvcreate", &["--config", "devices/scan_lvs=1", &lv_path])?;
    common::run("vgextend", &[target, &lv_path])?;

    println!("\n  ⚠ /etc/lvm/lvm.conf 에 scan_lvs = 1 설정 확인 (부팅 후에도 PV 인식되어야 함)");
    println!("\n=== 결과 ===");
    common::run("vgs", &[target])?;
    Ok(())
}

fn unstack(pv: &str, vg: &str, dry_run: bool) -> anyhow::Result<()> {
    println!("=== Stacked LVM 해체 ===");
    println!("  제거할 PV     : {pv}");
    println!("  대상 VG       : {vg}");

    // 1) PV 가 실제로 VG 에 속하는지 확인
    let pv_info = common::run_capture(
        "pvs",
        &["--config", "devices/scan_lvs=1", "--noheadings",
          "-o", "pv_name,vg_name,pv_used", "--units", "g", "--nosuffix", pv],
    )?;
    let p: Vec<&str> = pv_info.split_whitespace().collect();
    if p.len() < 3 {
        anyhow::bail!("PV {pv} 조회 실패");
    }
    let actual_vg = p[1];
    if actual_vg != vg {
        anyhow::bail!("PV {pv} 는 VG '{actual_vg}' 소속 — --vg 값 확인");
    }
    let pv_used_gib: f64 = p[2].parse().unwrap_or(0.0);
    let target_vg_free = vg_free_gib(vg).unwrap_or(0.0);

    println!("  PV 사용 중    : {:.2} G", pv_used_gib);
    println!("  대상 VG 여유  : {:.2} G", target_vg_free);

    // 2) pvmove 가능한지 (다른 PV 로 옮길 공간)
    if pv_used_gib > target_vg_free {
        anyhow::bail!(
            "다른 PV 로 옮길 공간 부족 — {:.2} G 필요, {:.2} G 여유.\n  선행: 대상 VG 에 공간 확보 (씬풀 축소 / 볼륨 정리)",
            pv_used_gib, target_vg_free
        );
    }

    // 3) 해체 절차
    if dry_run {
        println!("\n(dry-run) 실행 예정:");
        println!("  pvmove --config 'devices/scan_lvs=1' {pv}        # 다른 PV 로 익스텐트 이동");
        println!("  vgreduce --config 'devices/scan_lvs=1' {vg} {pv}");
        println!("  pvremove --config 'devices/scan_lvs=1' {pv}");
        println!("  lvremove -f {pv}                                 # 소스 VG 의 LV 삭제");
        return Ok(());
    }

    println!("\n[1/4] pvmove (온라인, 시간 걸릴 수 있음)");
    common::run("pvmove", &["--config", "devices/scan_lvs=1", pv])?;
    println!("\n[2/4] vgreduce");
    common::run("vgreduce", &["--config", "devices/scan_lvs=1", vg, pv])?;
    println!("\n[3/4] pvremove");
    common::run("pvremove", &["--config", "devices/scan_lvs=1", pv])?;
    println!("\n[4/4] 소스 LV 삭제");
    common::run("lvremove", &["-f", pv])?;

    println!("\n=== 해체 완료 ===");
    common::run("vgs", &[])?;
    Ok(())
}

fn plan_thin_reclaim(pool: &str, new_size: &str) -> anyhow::Result<()> {
    let (pool_vg, pool_name) = match pool.split_once('/') {
        Some(x) => x,
        None => anyhow::bail!("pool 은 'VG/LV' 형식 (예: pve/data)"),
    };
    println!("=== 씬풀 축소 플래닝 (실행 안 함) ===");
    println!("  씬풀         : {pool}");
    println!("  목표 크기    : {new_size}");
    println!();
    println!("⚠ 파괴적 작업. 다음 전제 필수:");
    println!("  1) {pool} 기반 전 VM/LXC 를 PBS 에 백업 + 복원 **리허설** 완료");
    println!("  2) 정비 윈도우 확보 (전 VM/LXC 정지 → 복원까지 2~4 시간 예상)");
    println!("  3) /etc/pve/storage.cfg 에서 local-lvm 항목 사전 제거");
    println!();
    println!("절차 (Proxmox 공식 권장):");
    println!("  # 1. 전 게스트 정지");
    println!("  for id in $(pct list | awk 'NR>1{{print $1}}'); do pct stop $id; done");
    println!("  for id in $(qm list | awk 'NR>1{{print $1}}'); do qm stop $id; done");
    println!();
    println!("  # 2. 씬풀 삭제");
    println!("  lvremove {pool}");
    println!();
    println!("  # 3. 이제 대상 VG 여유 생김. root/swap/기타 LV 확장 여지");
    println!("  # lvextend -L +XG /dev/{pool_vg}/root && resize2fs /dev/{pool_vg}/root");
    println!();
    println!("  # 4. 축소된 씬풀 재생성");
    println!("  lvcreate -L {new_size} -n {pool_name} {pool_vg}");
    let meta_hint = "1% of pool size, min 1G";
    println!("  lvconvert --type thin-pool --poolmetadatasize <N>G {pool}     # ({meta_hint})");
    println!();
    println!("  # 5. GUI 에서 local-lvm 재등록 또는 /etc/pve/storage.cfg 복구");
    println!();
    println!("  # 6. 각 VM/LXC 를 PBS 에서 복원");
    println!("  pxi-backup restore-all --storage local-lvm   # (pxi-backup 구현되면)");
    println!();
    println!("실제 실행은 의도적으로 구현 안 됨 — 사람 검토 후 수동 실행 권장.");
    Ok(())
}
