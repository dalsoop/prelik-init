//! prelik-ministack — MiniStack (로컬 AWS 에뮬레이터 = LocalStack).

use clap::{Parser, Subcommand};
use prelik_core::common;

#[derive(Parser)]
#[command(name = "prelik-ministack", about = "MiniStack (LocalStack AWS 에뮬레이터)")]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    Install { #[arg(long, default_value = "4566")] port: u16, #[arg(long)] data_dir: Option<String> },
    Uninstall { #[arg(long)] force: bool },
    Start, Stop, Restart, Status, Reset,
    Logs { #[arg(long)] follow: bool, #[arg(long)] tail: Option<String> },
    Update,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let dc = "/opt/ministack/docker-compose.yml";
    match cli.cmd {
        Cmd::Status => { common::run("docker", &["compose", "-f", dc, "ps"]); }
        Cmd::Start => { common::run("docker", &["compose", "-f", dc, "up", "-d"]); }
        Cmd::Stop => { common::run("docker", &["compose", "-f", dc, "down"]); }
        Cmd::Restart => { common::run("docker", &["compose", "-f", dc, "restart"]); }
        Cmd::Logs { follow, tail } => {
            let mut a = vec!["compose", "-f", dc, "logs"];
            if follow { a.push("-f"); }
            if let Some(t) = &tail { a.push("--tail"); a.push(t); }
            common::run("docker", &a);
        }
        _ => { println!("TODO: 미구현"); }
    }
    Ok(())
}
