//! prelik-infisical — Infisical 시크릿 관리 플랫폼.

use clap::{Parser, Subcommand};
use prelik_core::common;

#[derive(Parser)]
#[command(name = "prelik-infisical", about = "Infisical (시크릿 관리 플랫폼)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Docker Compose로 Infisical 설치
    Install { #[arg(long, default_value = "8082")] port: u16 },
    /// 제거
    Uninstall { #[arg(long)] force: bool },
    /// 시작
    Start,
    /// 중지
    Stop,
    /// 재시작
    Restart,
    /// 상태
    Status,
    /// 로그 확인
    Logs { #[arg(long)] follow: bool, #[arg(long)] tail: Option<String> },
    /// 업데이트
    Update,
    /// SMTP 설정 주입
    Smtp {
        #[arg(long, default_value = "10.0.50.122")] host: String,
        #[arg(long, default_value_t = 587)] port: u16,
        #[arg(long)] user: Option<String>,
        #[arg(long)] password: Option<String>,
        #[arg(long)] from: Option<String>,
        #[arg(long, default_value = "Infisical")] from_name: String,
        #[arg(long)] secure: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Status => { common::run("docker", &["compose", "-f", "/opt/infisical/docker-compose.yml", "ps"]); }
        Cmd::Start => { common::run("docker", &["compose", "-f", "/opt/infisical/docker-compose.yml", "up", "-d"]); }
        Cmd::Stop => { common::run("docker", &["compose", "-f", "/opt/infisical/docker-compose.yml", "down"]); }
        Cmd::Restart => { common::run("docker", &["compose", "-f", "/opt/infisical/docker-compose.yml", "restart"]); }
        Cmd::Logs { follow, tail } => {
            let mut args = vec!["compose", "-f", "/opt/infisical/docker-compose.yml", "logs"];
            if follow { args.push("-f"); }
            if let Some(t) = &tail { args.push("--tail"); args.push(t); }
            common::run("docker", &args);
        }
        _ => { println!("TODO: 이 서브커맨드는 아직 미구현. old phs에서 포팅 필요."); }
    }
    Ok(())
}
