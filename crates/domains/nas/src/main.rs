//! prelik-nas — 범용 NAS 마운트 관리.
//! SMB/CIFS + NFS, Synology/TrueNAS/일반 서버 전부 지원.

use clap::{Parser, Subcommand, ValueEnum};
use prelik_core::common;

#[derive(Parser)]
#[command(name = "prelik-nas", about = "NAS 마운트 관리 (SMB/NFS)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// NAS 공유 마운트
    Mount {
        /// NAS 서버 주소 (예: 192.168.1.100)
        #[arg(long)]
        host: String,
        /// 공유 이름 (SMB) 또는 export 경로 (NFS)
        #[arg(long)]
        share: String,
        /// 로컬 마운트 포인트 (예: /mnt/nas)
        #[arg(long)]
        target: String,
        /// 프로토콜
        #[arg(long, value_enum, default_value = "smb")]
        protocol: Protocol,
        /// SMB 사용자 (SMB에만 필요)
        #[arg(long)]
        user: Option<String>,
        /// SMB 비밀번호 (SMB에만 필요, 또는 /etc/prelik/.env의 NAS_PASSWORD)
        #[arg(long)]
        password: Option<String>,
        /// /etc/fstab에 영구 등록
        #[arg(long)]
        persist: bool,
    },
    /// 마운트 해제
    Unmount { target: String },
    /// 현재 마운트 목록 (NFS + CIFS만 필터)
    List,
    Doctor,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Protocol {
    Smb,
    Nfs,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Mount { host, share, target, protocol, user, password, persist } => {
            mount(&host, &share, &target, protocol, user.as_deref(), password.as_deref(), persist)
        }
        Cmd::Unmount { target } => unmount(&target),
        Cmd::List => {
            list();
            Ok(())
        }
        Cmd::Doctor => {
            doctor();
            Ok(())
        }
    }
}

fn mount(
    host: &str, share: &str, target: &str,
    protocol: Protocol,
    user: Option<&str>, password: Option<&str>,
    persist: bool,
) -> anyhow::Result<()> {
    println!("=== NAS 마운트 ({protocol:?}) ===");
    println!("  source: {host}:{share}");
    println!("  target: {target}");

    common::run_bash(&format!("sudo mkdir -p {target}"))?;

    let (mount_cmd, fstab_line) = match protocol {
        Protocol::Smb => {
            // SMB 크리덴셜: 인자 > .env > prompt 에러
            let user = user.map(String::from).unwrap_or_else(|| read_env("NAS_USER"));
            let password = password.map(String::from).unwrap_or_else(|| read_env("NAS_PASSWORD"));
            if user.is_empty() {
                anyhow::bail!("SMB 마운트에는 --user 또는 NAS_USER 환경변수 필요");
            }
            let options = if password.is_empty() {
                format!("username={user},vers=3.0")
            } else {
                format!("username={user},password={password},vers=3.0")
            };
            let cmd = format!(
                "sudo mount -t cifs -o {} //{}/{} {}",
                options, host, share, target
            );
            let fstab = format!(
                "//{host}/{share} {target} cifs {options},_netdev,nofail 0 0"
            );
            (cmd, fstab)
        }
        Protocol::Nfs => {
            let cmd = format!("sudo mount -t nfs {}:{} {}", host, share, target);
            let fstab = format!(
                "{host}:{share} {target} nfs _netdev,nofail 0 0"
            );
            (cmd, fstab)
        }
    };

    common::run_bash(&mount_cmd)?;
    println!("✓ 마운트 완료");

    if persist {
        // fstab에 이미 있는지 확인
        let check = format!("grep -qF '{target}' /etc/fstab");
        if common::run_bash(&check).is_ok() {
            println!("  ⊘ /etc/fstab에 이미 등록됨 — 건너뜀");
        } else {
            common::run_bash(&format!("echo '{}' | sudo tee -a /etc/fstab >/dev/null", fstab_line))?;
            println!("  ✓ /etc/fstab 등록 (재부팅 후에도 유지)");
        }
    }
    Ok(())
}

fn unmount(target: &str) -> anyhow::Result<()> {
    println!("=== 마운트 해제: {target} ===");
    common::run_bash(&format!("sudo umount {target}"))?;
    // fstab에서 해당 라인 제거할지는 사용자 판단 — 자동 제거 안 함 (의도적 보수적)
    let check = format!("grep -qF '{target}' /etc/fstab && echo 'warn'");
    if common::run_bash(&check).is_ok() {
        println!("  ⚠ /etc/fstab에 등록 있음. 영구 제거: sudo sed -i '\\|{target}|d' /etc/fstab");
    }
    println!("✓ 해제 완료");
    Ok(())
}

fn list() {
    println!("=== NAS 마운트 목록 (cifs + nfs) ===");
    if let Ok(out) = common::run_bash("mount | grep -E 'type (cifs|nfs)'") {
        if out.trim().is_empty() {
            println!("  (없음)");
        } else {
            for line in out.lines() {
                println!("  {line}");
            }
        }
    } else {
        println!("  (없음)");
    }
}

fn read_env(key: &str) -> String {
    let paths = ["/etc/prelik/.env", "/etc/proxmox-host-setup/.env"];
    for p in paths {
        if let Ok(raw) = std::fs::read_to_string(p) {
            for line in raw.lines() {
                if let Some(v) = line.strip_prefix(&format!("{key}=")) {
                    return v.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    String::new()
}

fn doctor() {
    println!("=== prelik-nas doctor ===");
    for (name, cmd) in &[
        ("mount", "mount"),
        ("mount.cifs (cifs-utils)", "mount.cifs"),
        ("mount.nfs (nfs-common)", "mount.nfs"),
    ] {
        println!("  {} {name}", if common::has_cmd(cmd) { "✓" } else { "✗" });
    }
    println!("\n필요시 설치: sudo apt install -y cifs-utils nfs-common");
}
