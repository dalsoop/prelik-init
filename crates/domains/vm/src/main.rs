//! prelik-vm — Proxmox QEMU VM 관리 (qm 래퍼).
//! LXC와 별개. vzdump는 LXC와 공통.

use clap::{Parser, Subcommand};
use prelik_core::common;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "prelik-vm", about = "Proxmox QEMU VM 관리")]
struct Cli {
    /// list/status를 JSON으로 출력 (자동화/CI 친화)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Serialize, Debug, PartialEq)]
struct VmRow {
    vmid: String,
    name: String,
    status: String,
    mem_mb: String,
    disk_gb: String,
    pid: Option<String>,
}

// upstream qemu-server VM 상태값:
//   running, stopped, paused, suspended, prelaunch
// (LXC는 paused/suspended 안 씀. VM은 ACPI suspend 등으로 가능)
const STATUS_KNOWN: &[&str] = &["running", "stopped", "paused", "suspended", "prelaunch"];

// 순수 파서 — qm list 출력. 헤더 + 데이터 라인.
// "VMID NAME STATUS MEM(MB) BOOTDISK(GB) PID" → 5컬럼(PID 0/누락) 또는 6컬럼.
fn parse_qm_list(text: &str) -> anyhow::Result<Vec<VmRow>> {
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        if line.trim().is_empty() { continue; }
        let p: Vec<&str> = line.split_whitespace().collect();
        let row = match p.len() {
            5 => VmRow {
                vmid: p[0].into(), name: p[1].into(), status: p[2].into(),
                mem_mb: p[3].into(), disk_gb: p[4].into(), pid: None,
            },
            6 => VmRow {
                vmid: p[0].into(), name: p[1].into(), status: p[2].into(),
                mem_mb: p[3].into(), disk_gb: p[4].into(),
                pid: if p[5] == "0" { None } else { Some(p[5].into()) },
            },
            _ => anyhow::bail!("qm list 라인 파싱 실패 (컬럼 {}개): {line:?}", p.len()),
        };
        // status JSON 경로와 동일한 whitelist 계약 — drift 거부.
        if !STATUS_KNOWN.contains(&row.status.as_str()) {
            anyhow::bail!(
                "qm list 행의 status가 알 수 없는 형태: {:?} (허용: {STATUS_KNOWN:?})",
                row.status
            );
        }
        rows.push(row);
    }
    Ok(rows)
}

// "status: <value>\n" 단일 라인 raw 검증. lxc와 동일 패턴, whitelist만 다름.
fn parse_qm_status(raw: &str) -> anyhow::Result<&str> {
    let body = raw.strip_suffix('\n').unwrap_or(raw);
    if body.contains('\n') {
        anyhow::bail!("qm status 출력이 단일 라인이 아님: {raw:?}");
    }
    let value = body.strip_prefix("status: ")
        .ok_or_else(|| anyhow::anyhow!("qm status 출력 형식이 'status: <value>' 아님: {raw:?}"))?;
    if !STATUS_KNOWN.contains(&value) {
        anyhow::bail!("qm status 값이 알 수 없는 형태: {value:?} (허용: {STATUS_KNOWN:?})");
    }
    Ok(value)
}

#[derive(Subcommand)]
enum Cmd {
    /// VM 목록
    List,
    /// VM 상태
    Status { vmid: String },
    /// VM 시작
    Start { vmid: String },
    /// VM 정지 (graceful shutdown, 타임아웃 시 강제)
    Stop { vmid: String },
    /// VM 재시작 (reset)
    Reboot { vmid: String },
    /// VM 삭제 (purge — 디스크까지)
    Delete {
        vmid: String,
        #[arg(long)]
        force: bool,
    },
    /// VM 백업
    Backup {
        vmid: String,
        #[arg(long, default_value = "local")]
        storage: String,
        #[arg(long, default_value = "snapshot")]
        mode: String,
    },
    /// VM 리소스 변경
    Resize {
        vmid: String,
        #[arg(long)]
        cores: Option<String>,
        #[arg(long)]
        memory: Option<String>,
    },
    /// 콘솔 접속 (qm terminal)
    Console { vmid: String },
    Doctor,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let json = cli.json;
    if !matches!(cli.cmd, Cmd::Doctor) && !common::has_cmd("qm") {
        anyhow::bail!("qm 없음 — Proxmox 호스트에서만 동작");
    }
    match cli.cmd {
        Cmd::List => list(json),
        Cmd::Status { vmid } => status(&vmid, json),
        Cmd::Start { vmid } => {
            common::run("qm", &["start", &vmid])?;
            println!("✓ VM {vmid} 시작");
            Ok(())
        }
        Cmd::Stop { vmid } => {
            common::run("qm", &["shutdown", &vmid, "--timeout", "60", "--forceStop", "1"])?;
            println!("✓ VM {vmid} 정지");
            Ok(())
        }
        Cmd::Reboot { vmid } => {
            common::run("qm", &["reboot", &vmid])?;
            println!("✓ VM {vmid} 재시작");
            Ok(())
        }
        Cmd::Delete { vmid, force } => {
            if !force {
                anyhow::bail!("삭제는 --force 필요 (복구 불가)");
            }
            let status = common::run("qm", &["status", &vmid]).unwrap_or_default();
            if status.contains("running") {
                common::run("qm", &["stop", &vmid])?;
            }
            common::run("qm", &["destroy", &vmid, "--purge", "1"])?;
            println!("✓ VM {vmid} 삭제");
            Ok(())
        }
        Cmd::Backup { vmid, storage, mode } => {
            println!("=== VM {vmid} 백업 ===");
            common::run("vzdump", &[&vmid, "--storage", &storage, "--mode", &mode, "--compress", "zstd"])?;
            println!("✓ 백업 완료");
            Ok(())
        }
        Cmd::Resize { vmid, cores, memory } => {
            if cores.is_none() && memory.is_none() {
                anyhow::bail!("--cores 또는 --memory 최소 하나");
            }
            if let Some(c) = cores {
                common::run("qm", &["set", &vmid, "--cores", &c])?;
                println!("  ✓ cores: {c}");
            }
            if let Some(m) = memory {
                common::run("qm", &["set", &vmid, "--memory", &m])?;
                println!("  ✓ memory: {m} MB");
            }
            Ok(())
        }
        Cmd::Console { vmid } => {
            let status = std::process::Command::new("qm")
                .args(["terminal", &vmid])
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Cmd::Doctor => {
            doctor();
            Ok(())
        }
    }
}

fn list(json: bool) -> anyhow::Result<()> {
    let out = common::run("qm", &["list"])?;
    if !json {
        println!("{out}");
        return Ok(());
    }
    let rows = parse_qm_list(&out)?;
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

fn status(vmid: &str, json: bool) -> anyhow::Result<()> {
    if !json {
        let out = common::run("qm", &["status", vmid])?;
        println!("{out}");
        return Ok(());
    }
    // raw stdout — common::run의 trim 회피.
    let output = std::process::Command::new("qm").args(["status", vmid]).output()?;
    if !output.status.success() {
        anyhow::bail!("qm status {vmid} 실패: {}", String::from_utf8_lossy(&output.stderr));
    }
    let raw = String::from_utf8(output.stdout)?;
    let value = parse_qm_status(&raw)?;
    let payload = serde_json::json!({ "vmid": vmid, "status": value });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn doctor() {
    println!("=== prelik-vm doctor ===");
    for (name, cmd) in &[("qm", "qm"), ("vzdump", "vzdump"), ("pvesh", "pvesh")] {
        println!("  {} {name}", if common::has_cmd(cmd) { "✓" } else { "✗" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- parse_qm_list -----

    #[test]
    fn list_running_with_pid() {
        let text = "VMID NAME STATUS MEM(MB) BOOTDISK(GB) PID\n\
                    100 web running 2048 32 1234\n";
        let rows = parse_qm_list(text).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], VmRow {
            vmid: "100".into(), name: "web".into(), status: "running".into(),
            mem_mb: "2048".into(), disk_gb: "32".into(), pid: Some("1234".into()),
        });
    }

    #[test]
    fn list_stopped_pid_zero_normalized_to_none() {
        let text = "VMID NAME STATUS MEM DISK PID\n\
                    101 db stopped 4096 64 0\n";
        let rows = parse_qm_list(text).unwrap();
        assert_eq!(rows[0].pid, None);
    }

    #[test]
    fn list_5_columns_no_pid() {
        // qm이 PID 컬럼 자체를 생략하는 경우 (오래된 출력)
        let text = "VMID NAME STATUS MEM DISK\n\
                    102 stopped-vm stopped 1024 8\n";
        let rows = parse_qm_list(text).unwrap();
        assert_eq!(rows[0].pid, None);
        assert_eq!(rows[0].name, "stopped-vm");
    }

    #[test]
    fn list_skips_empty_lines() {
        let text = "VMID NAME STATUS MEM DISK PID\n\
                    100 a running 2048 32 1\n\
                    \n\
                    101 b stopped 4096 64 0\n";
        assert_eq!(parse_qm_list(text).unwrap().len(), 2);
    }

    #[test]
    fn list_fails_on_too_few_columns() {
        let text = "VMID NAME STATUS MEM\n\
                    100 a running 2048\n";
        assert!(parse_qm_list(text).is_err());
    }

    #[test]
    fn list_fails_on_too_many_columns() {
        let text = "VMID NAME STATUS MEM DISK PID EXTRA\n\
                    100 a running 2048 32 1234 x\n";
        assert!(parse_qm_list(text).is_err());
    }

    #[test]
    fn list_only_header_returns_empty() {
        assert!(parse_qm_list("VMID NAME STATUS MEM DISK PID").unwrap().is_empty());
    }

    #[test]
    fn list_fails_on_unknown_status() {
        // status whitelist 위반 — qm이 'unknown'을 emit하면 (LXC 전용 fallback)
        // VM 도메인에선 거부해야 함. status JSON 경로와 동일 계약.
        let text = "VMID NAME STATUS MEM DISK PID\n\
                    100 web unknown 2048 32 0\n";
        assert!(parse_qm_list(text).is_err());
    }

    #[test]
    fn list_fails_on_drifted_status() {
        let text = "VMID NAME STATUS MEM DISK PID\n\
                    100 web RUNNING 2048 32 0\n"; // 대문자
        assert!(parse_qm_list(text).is_err());
    }

    // ----- parse_qm_status -----

    #[test]
    fn status_running() {
        assert_eq!(parse_qm_status("status: running\n").unwrap(), "running");
    }

    #[test]
    fn status_paused() {
        assert_eq!(parse_qm_status("status: paused\n").unwrap(), "paused");
    }

    #[test]
    fn status_suspended() {
        assert_eq!(parse_qm_status("status: suspended\n").unwrap(), "suspended");
    }

    #[test]
    fn status_prelaunch() {
        assert_eq!(parse_qm_status("status: prelaunch\n").unwrap(), "prelaunch");
    }

    #[test]
    fn status_stopped_no_trailing_newline() {
        assert_eq!(parse_qm_status("status: stopped").unwrap(), "stopped");
    }

    #[test]
    fn status_rejects_extra_lines() {
        assert!(parse_qm_status("status: running\nwarning: x\n").is_err());
    }

    #[test]
    fn status_rejects_missing_prefix() {
        assert!(parse_qm_status("state: running\n").is_err());
        assert!(parse_qm_status(" status: running\n").is_err());
    }

    #[test]
    fn status_rejects_value_drift() {
        assert!(parse_qm_status("status: \n").is_err());
        assert!(parse_qm_status("status:  running\n").is_err());
        assert!(parse_qm_status("status: running \n").is_err());
        assert!(parse_qm_status("status: unknown\n").is_err()); // qm은 unknown 안 emit (LXC 전용)
    }
}
