//! 공통 헬퍼
use std::process::Command;

pub fn run(cmd: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(cmd).args(args).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "{} {:?} 실패: {}",
            cmd, args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run_bash(script: &str) -> anyhow::Result<String> {
    run("bash", &["-c", script])
}

pub fn has_cmd(name: &str) -> bool {
    which::which(name).is_ok()
}

/// run() 변형: 비밀이 args에 포함될 때 사용. 실패 시 argv는 메시지에 노출되지 않음.
/// stdout/stderr는 자식이 직접 상속해서 출력 — 호출자는 결과를 받지 못하지만
/// 자격증명이 anyhow chain에 영구 기록되는 것을 막는다.
pub fn run_secret(cmd: &str, args: &[&str], context: &str) -> anyhow::Result<()> {
    let status = Command::new(cmd).args(args).status()
        .map_err(|e| anyhow::anyhow!("{cmd} spawn 실패: {e}"))?;
    if !status.success() {
        anyhow::bail!(
            "{context} 실패 (exit {}). 자격증명 보호를 위해 argv는 메시지에 포함하지 않음.",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
