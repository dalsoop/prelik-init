//! prelik-license — Keygen CE 라이선스 활성화/상태/해제.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "prelik-license", about = "라이선스 관리 (Keygen CE)")]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    /// 라이선스 키로 이 기기 활성화
    Activate { key: String },
    /// 현재 라이선스 상태
    Status,
    /// 이 기기 활성화 해제
    Deactivate,
    /// 서버에 heartbeat 전송
    CheckIn,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // TODO: port license/mod.rs from proxmox-host-setup
    // Keygen API 연동 코드는 /tmp/proxmox-host-setup/src/license/mod.rs 참조
    match cli.cmd {
        Cmd::Activate { key } => println!("activate: {} (TODO: Keygen API)", key),
        Cmd::Status => println!("status (TODO)"),
        Cmd::Deactivate => println!("deactivate (TODO)"),
        Cmd::CheckIn => println!("check-in (TODO)"),
    }
    Ok(())
}
