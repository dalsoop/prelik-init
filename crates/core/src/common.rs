use anyhow::{Context, Result};
use std::process::Command;

/// 외부 명령 실행 — stdout/stderr 상속 (interactive)
pub fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("{cmd} 실행 실패"))?;
    if !status.success() {
        anyhow::bail!("{cmd} exited with {}", status);
    }
    Ok(())
}

/// 외부 명령 실행 — stdout 캡처
pub fn run_capture(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("{cmd} 실행 실패"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{cmd} 실패: {stderr}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 명령 존재 여부 확인
pub fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `command_exists`의 호환 alias — 레거시 코드 대응
#[inline]
pub fn has_cmd(cmd: &str) -> bool {
    command_exists(cmd)
}

/// bash -lc 스크립트 실행 + stdout 캡처
pub fn run_bash(script: &str) -> Result<String> {
    run_capture("bash", &["-lc", script])
}

/// pct exec 래퍼 — LXC 안에서 명령 실행 (stdout 캡처)
pub fn pct_exec(vmid: &str, cmd_args: &[&str]) -> Result<String> {
    let mut args = vec!["exec", vmid, "--"];
    args.extend_from_slice(cmd_args);
    run_capture("pct", &args)
}

/// pct exec 래퍼 — stdout/stderr 상속 (interactive)
pub fn pct_exec_passthrough(vmid: &str, cmd_args: &[&str]) -> Result<()> {
    let mut args = vec!["exec", vmid, "--"];
    args.extend_from_slice(cmd_args);
    run("pct", &args)
}

/// LXC 실행 상태 확인 + 미실행 시 시작
pub fn ensure_lxc_running(vmid: &str) -> Result<()> {
    let status = run_capture("pct", &["status", vmid])?;
    if !status.contains("running") {
        run("pct", &["start", vmid])?;
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    Ok(())
}

/// `run_capture`의 간편 별칭 — 레거시 코드가 `common::run(...).trim()` 형태로 사용하던 패턴 복원
#[inline]
pub fn run_str(cmd: &str, args: &[&str]) -> Result<String> {
    run_capture(cmd, args)
}
