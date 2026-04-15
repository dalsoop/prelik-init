//! prelik-workspace — 개발자 작업 환경 초기화.
//! tmux 기본 conf + zsh/bash alias + fzf/bat/eza 등 CLI 도구.

use clap::{Parser, Subcommand};
use prelik_core::common;
use std::fs;

#[derive(Parser)]
#[command(name = "prelik-workspace", about = "작업 환경 (tmux + shell 도구)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// tmux 기본 설정 배포 (~/.tmux.conf)
    TmuxSetup,
    /// shell alias + 편의 도구 (~/.bashrc.d/prelik.sh)
    ShellSetup,
    /// 현재 작업 환경 상태
    Status,
    Doctor,
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::TmuxSetup => tmux_setup(),
        Cmd::ShellSetup => shell_setup(),
        Cmd::Status => {
            status();
            Ok(())
        }
        Cmd::Doctor => {
            doctor();
            Ok(())
        }
    }
}

const TMUX_CONF: &str = "# prelik-workspace tmux conf — 공식 기본 설정
set -g default-terminal \"tmux-256color\"
set -ga terminal-overrides \",*256col*:Tc\"
set -g mouse on
set -g base-index 1
setw -g pane-base-index 1
set -g renumber-windows on
set -g history-limit 100000
set -g status-interval 5
set -sg escape-time 0

# 키바인딩
bind | split-window -h -c \"#{pane_current_path}\"
bind - split-window -v -c \"#{pane_current_path}\"
bind c new-window -c \"#{pane_current_path}\"
bind r source-file ~/.tmux.conf \\; display \"Reloaded!\"

# vim 스타일 pane 이동
bind h select-pane -L
bind j select-pane -D
bind k select-pane -U
bind l select-pane -R

# 상태줄
set -g status-position bottom
set -g status-justify left
set -g status-bg '#1e1e2e'
set -g status-fg '#cdd6f4'
set -g status-left-length 30
set -g status-right-length 50
";

const SHELL_RC: &str = "# prelik-workspace shell alias
# 자동 생성됨 — ~/.bashrc에서 source ~/.bashrc.d/prelik.sh

# 기본 편의
alias ll='ls -lah'
alias ..='cd ..'
alias ...='cd ../..'
alias grep='grep --color=auto'

# 안전망
alias rm='rm -i'
alias cp='cp -i'
alias mv='mv -i'

# 대체 도구 (있을 때만)
if command -v bat >/dev/null; then alias cat='bat --paging=never'; fi
if command -v eza >/dev/null; then alias ls='eza'; fi
if command -v fd >/dev/null; then alias find='fd'; fi

# tmux
alias t='tmux'
alias ta='tmux attach -t'
alias tn='tmux new -s'
alias tl='tmux ls'

# git
alias g='git'
alias gs='git status'
alias gd='git diff'
alias gl='git log --oneline --graph --decorate -20'
";

fn tmux_setup() -> anyhow::Result<()> {
    println!("=== tmux setup ===");
    if !common::has_cmd("tmux") {
        anyhow::bail!("tmux 미설치 — sudo apt install tmux");
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("HOME 미설정"))?;
    let conf_path = home.join(".tmux.conf");

    if conf_path.exists() {
        // 기존 백업
        let backup = home.join(format!(".tmux.conf.prelik-backup-{}",
            common::run("date", &["+%Y%m%d-%H%M%S"]).unwrap_or_default().trim()));
        fs::copy(&conf_path, &backup)?;
        println!("  백업: {}", backup.display());
    }

    fs::write(&conf_path, TMUX_CONF)?;
    println!("✓ {} 배포 완료", conf_path.display());
    println!("  적용: tmux source-file ~/.tmux.conf (기존 세션)");
    Ok(())
}

fn shell_setup() -> anyhow::Result<()> {
    println!("=== shell alias setup ===");
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("HOME 미설정"))?;
    let rc_dir = home.join(".bashrc.d");
    fs::create_dir_all(&rc_dir)?;
    let rc_path = rc_dir.join("prelik.sh");

    fs::write(&rc_path, SHELL_RC)?;
    println!("✓ {} 배포", rc_path.display());

    // ~/.bashrc에 source 줄 추가 (없으면)
    let bashrc = home.join(".bashrc");
    let source_line = "for f in ~/.bashrc.d/*.sh; do [ -r \"$f\" ] && source \"$f\"; done";
    let existing = fs::read_to_string(&bashrc).unwrap_or_default();
    if !existing.contains("bashrc.d") {
        let appended = format!("{existing}\n# prelik-workspace\n{source_line}\n");
        fs::write(&bashrc, appended)?;
        println!("  ~/.bashrc에 source 줄 추가");
    } else {
        println!("  ⊘ ~/.bashrc에 이미 source 줄 있음");
    }

    println!("  적용: source ~/.bashrc (새 셸 또는)");
    Ok(())
}

fn status() {
    println!("=== workspace status ===");
    let home = dirs::home_dir();
    if let Some(h) = home {
        let tmux = h.join(".tmux.conf");
        println!("  tmux.conf: {}", if tmux.exists() { "✓" } else { "✗ (prelik run workspace tmux-setup)" });
        let alias = h.join(".bashrc.d/prelik.sh");
        println!("  shell alias: {}", if alias.exists() { "✓" } else { "✗ (prelik run workspace shell-setup)" });
    }

    if let Ok(out) = common::run_bash("tmux ls 2>/dev/null | wc -l") {
        println!("  tmux 세션:   {}개", out.trim());
    }
}

fn doctor() {
    println!("=== prelik-workspace doctor ===");
    for (name, cmd) in &[
        ("tmux", "tmux"),
        ("bat (선택)", "bat"),
        ("eza (선택)", "eza"),
        ("fd (선택)", "fd"),
        ("fzf (선택)", "fzf"),
    ] {
        println!("  {} {name}", if common::has_cmd(cmd) { "✓" } else { "✗" });
    }
    println!("\n선택 도구 설치: sudo apt install -y bat eza fd-find fzf");
}
