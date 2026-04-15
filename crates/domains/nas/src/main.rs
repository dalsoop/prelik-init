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

    // mkdir은 argv로 직접 (shell interpolation 회피)
    common::run("sudo", &["mkdir", "-p", target])?;

    match protocol {
        Protocol::Smb => mount_smb(host, share, target, user, password, persist),
        Protocol::Nfs => mount_nfs(host, share, target, persist),
    }
}

fn mount_smb(
    host: &str, share: &str, target: &str,
    user: Option<&str>, password: Option<&str>,
    persist: bool,
) -> anyhow::Result<()> {
    let user = user.map(String::from).unwrap_or_else(|| read_env("NAS_USER"));
    let password = password.map(String::from).unwrap_or_else(|| read_env("NAS_PASSWORD"));
    if user.is_empty() {
        anyhow::bail!("SMB 마운트에는 --user 또는 NAS_USER 환경변수 필요");
    }

    // credentials 파일 경로 (host-share 키로 고유화)
    // 비밀번호를 ps/cmdline/fstab에 노출하지 않음
    let safe_name = format!("{host}_{share}")
        .chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>();
    let cred_path = format!("/etc/cifs-credentials/{safe_name}");

    // 디렉토리 + 크리덴셜 파일 작성 (0600, root:root) — mktemp 경유
    common::run("sudo", &["mkdir", "-p", "/etc/cifs-credentials"])?;
    common::run("sudo", &["chmod", "700", "/etc/cifs-credentials"])?;

    // tempfile로 로컬 생성 후 sudo install로 원자적 이동
    let (tmp, _guard) = secure_tempfile()?;
    let content = if password.is_empty() {
        format!("username={user}\n")
    } else {
        format!("username={user}\npassword={password}\n")
    };
    std::fs::write(&tmp, content)?;
    common::run("sudo", &[
        "install", "-m", "600", "-o", "root", "-g", "root",
        &tmp, &cred_path,
    ])?;

    // mount 호출 — 모든 인자를 argv로 직접
    let source = format!("//{host}/{share}");
    let options = format!("credentials={cred_path},vers=3.0,iocharset=utf8,_netdev,nofail");
    common::run("sudo", &[
        "mount", "-t", "cifs", "-o", &options, &source, target,
    ])?;
    println!("✓ 마운트 완료 (credentials: {cred_path}, 0600)");

    if persist {
        let fstab_line = format!("{source} {target} cifs {options} 0 0");
        fstab_add(target, &fstab_line)?;
    }
    Ok(())
}

fn mount_nfs(host: &str, share: &str, target: &str, persist: bool) -> anyhow::Result<()> {
    let source = format!("{host}:{share}");
    common::run("sudo", &["mount", "-t", "nfs", &source, target])?;
    println!("✓ 마운트 완료");

    if persist {
        let fstab_line = format!("{source} {target} nfs _netdev,nofail 0 0");
        fstab_add(target, &fstab_line)?;
    }
    Ok(())
}

fn fstab_add(target: &str, fstab_line: &str) -> anyhow::Result<()> {
    // 이미 있으면 건너뜀 — grep -F로 고정문자열 매칭
    let check = std::process::Command::new("grep")
        .args(["-qF", target, "/etc/fstab"])
        .status();
    if check.ok().map(|s| s.success()).unwrap_or(false) {
        println!("  ⊘ /etc/fstab에 이미 등록됨 — 건너뜀");
        return Ok(());
    }
    // tempfile로 append 안전하게
    let (tmp, _g) = secure_tempfile()?;
    let current = std::fs::read_to_string("/etc/fstab").unwrap_or_default();
    let appended = if current.ends_with('\n') || current.is_empty() {
        format!("{current}{fstab_line}\n")
    } else {
        format!("{current}\n{fstab_line}\n")
    };
    std::fs::write(&tmp, appended)?;
    common::run("sudo", &["install", "-m", "644", "-o", "root", "-g", "root", &tmp, "/etc/fstab"])?;
    println!("  ✓ /etc/fstab 등록 (재부팅 후에도 유지)");
    Ok(())
}

/// mktemp 0600 + Drop 가드
fn secure_tempfile() -> anyhow::Result<(String, TempGuard)> {
    let out = common::run("mktemp", &["-t", "prelik.XXXXXXXX"])?;
    let tmp = out.trim().to_string();
    let guard = TempGuard(tmp.clone());
    common::run("chmod", &["600", &tmp])?;
    Ok((tmp, guard))
}

struct TempGuard(String);
impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn unmount(target: &str) -> anyhow::Result<()> {
    println!("=== 마운트 해제: {target} ===");
    common::run("sudo", &["umount", target])?;
    // fstab에서 해당 라인 제거할지는 사용자 판단 — 자동 제거 안 함
    let check = std::process::Command::new("grep")
        .args(["-qF", target, "/etc/fstab"])
        .status();
    if check.ok().map(|s| s.success()).unwrap_or(false) {
        println!("  ⚠ /etc/fstab에 등록 있음. 영구 제거: sudo sed -i \"\\|{target}|d\" /etc/fstab");
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
