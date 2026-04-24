//! pxi-vaultwarden — Vaultwarden(self-hosted Bitwarden) 전체 라이프사이클 도메인.
//!
//! 지원 범위:
//!   - 설치         : install-systemd / install-backup-timer / install-env
//!                    / install-binary / install-web-vault / bootstrap
//!   - reconcile    : domain-set / smtp-mailgun
//!   - 운영         : status / logs / restart / doctor / backup
//!   - 업그레이드   : upgrade (db 스냅샷 + 재빌드 + 재시작)
//!   - 클라이언트   : bw-install (버전 고정) / bw-verify (E2E 검증)
//!
//! LXC 는 사전에 `pxi run lxc create` 로 생성돼 있어야 하며, vmid 기본값은 50118.
//! 기본 설정은 CF Email Sending same-zone 제한을 피해 Mailgun API SMTP shim
//! (`pxi run mail mailgun-shim-install`) 을 SMTP relay 로 사용.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use pxi_core::common;
use std::process::Command;

const DEFAULT_VMID: u32 = 50118;
const DEFAULT_DOMAIN: &str = "https://vaultwarden.50.internal.kr";
const MAILGUN_SHIM_HOST: &str = "10.0.50.122"; // LINT_ALLOW: CF mail proxy 고정 주소 — control-plane CF_MAIL_PROXY_HOST 와 일치
const MAILGUN_SHIM_PORT: u16 = 2526;
const MAILGUN_FROM: &str = "devops@ranode.net";

#[derive(Parser)]
#[command(
    name = "pxi-vaultwarden",
    about = "Vaultwarden (self-hosted Bitwarden) reconcile"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// systemctl status vaultwarden
    Status {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
    },
    /// journalctl 로그
    Logs {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        #[arg(long)]
        follow: bool,
        #[arg(long, default_value_t = 50)]
        tail: u32,
    },
    /// Vaultwarden 재시작
    Restart {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
    },
    /// 설정 점검 (DOMAIN / SMTP / TLS / backup)
    Doctor {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
    },
    /// invite/verify 메일 링크 생성용 DOMAIN 설정 (config.json 반영)
    DomainSet {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        #[arg(long, default_value = DEFAULT_DOMAIN)]
        url: String,
    },
    /// SMTP 를 mailgun-smtp-proxy(2526) 경유로 설정 — 같은 CF zone 수신 가능
    SmtpMailgun {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        #[arg(long, default_value = MAILGUN_SHIM_HOST)]
        host: String,
        #[arg(long, default_value_t = MAILGUN_SHIM_PORT)]
        port: u16,
        #[arg(long, default_value = MAILGUN_FROM)]
        from: String,
    },
    /// Bitwarden CLI 설치. Vaultwarden 1.35.7+ 는 2026.x 와 호환 확인됨
    /// (이전 1.34 에서는 userDecryptionOptions 누락으로 실패했음).
    /// 기본 2024.7.2 는 하위 호환 안전판. `--rbw` 면 Rust 구현체.
    BwInstall {
        #[arg(long)]
        rbw: bool,
        #[arg(long, default_value = "2024.7.2")]
        version: String,
    },
    /// 일일 sqlite 백업 systemd timer 상태 확인
    Backup {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
    },
    /// /etc/systemd/system/vaultwarden.service 를 표준 템플릿으로 install.
    /// 바이너리(/opt/vaultwarden/bin/vaultwarden)·유저·data 디렉토리는 전제.
    InstallSystemd {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
    },
    /// vaultwarden-backup.service + .timer (daily) + /opt/vaultwarden/backup.sh 설치.
    InstallBackupTimer {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
    },
    /// 기본 /opt/vaultwarden/.env 를 생성 (DOMAIN, ROCKET 세팅, mailgun-shim SMTP).
    /// ADMIN_TOKEN 은 `--admin-token` 또는 control-plane/.env 의 VAULTWARDEN_ADMIN_TOKEN.
    InstallEnv {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        #[arg(long, default_value = DEFAULT_DOMAIN)]
        url: String,
        #[arg(long)]
        admin_token: Option<String>,
    },
    /// LXC 내부에서 Vaultwarden 소스 빌드 → /opt/vaultwarden/bin/vaultwarden 배치.
    /// rustup + apt deps + git clone + cargo build --release 로 수분 소요.
    /// 이미 있으면 --force 없이 skip.
    InstallBinary {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        /// vaultwarden git tag 또는 branch (기본: latest 릴리스 태그)
        #[arg(long)]
        version: Option<String>,
        /// cargo features (sqlite / mysql / postgresql). 기본 sqlite
        #[arg(long, default_value = "sqlite")]
        features: String,
        /// 이미 있어도 재빌드
        #[arg(long)]
        force: bool,
    },
    /// bw_web_builds 의 pre-built web-vault tarball 을 /opt/vaultwarden/web-vault 로 배치.
    InstallWebVault {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        /// git tag (예: v2026.2.0). 기본: latest
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// 신규 LXC 에 원샷으로: install-env + install-systemd + install-backup-timer
    /// + install-web-vault + install-binary (필요 시) → start.
    /// 이미 있는 단계는 각자 멱등 처리.
    Bootstrap {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        #[arg(long, default_value = DEFAULT_DOMAIN)]
        url: String,
        #[arg(long)]
        admin_token: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        web_vault_version: Option<String>,
        /// 바이너리 빌드 step 건너뜀 (이미 /opt/vaultwarden/bin/vaultwarden 있을 때)
        #[arg(long)]
        skip_binary: bool,
    },
    /// 기존 설치를 새 릴리스로 업그레이드:
    ///   1) db.sqlite3 스냅샷 백업
    ///   2) install-binary --force (새 태그 or latest)
    ///   3) install-web-vault --force (새 태그 or latest)
    ///   4) systemctl restart vaultwarden
    /// --dry-run 이면 현재 / 목표 태그만 비교 출력.
    Upgrade {
        #[arg(long, default_value_t = DEFAULT_VMID)]
        vmid: u32,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        web_vault_version: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// bw CLI 로 로그인/언락/동기화까지 end-to-end 검증.
    /// control-plane/.env 의 VAULTWARDEN_URL / _EMAIL / _MASTER_PASSWORD 사용.
    /// Vaultwarden 서버와 bw 버전 호환성을 즉시 감지.
    BwVerify {
        /// BITWARDENCLI_APPDATA_DIR (세션 보존용 고정 경로)
        #[arg(long, default_value = "/root/.config/vaultwarden-cli")]
        appdata: String,
    },

    /// Vaultwarden 의 homelab-env-keys 아이템에서 DOTENV_PRIVATE_KEY 값 출력 (stdout).
    /// 스크립트에서: KEY=$(pxi run vaultwarden env-key-get)
    EnvKeyGet {
        #[arg(long, default_value = "/root/.config/vaultwarden-cli")]
        appdata: String,
        /// 꺼낼 필드명 (기본: DOTENV_PRIVATE_KEY)
        #[arg(long, default_value = "DOTENV_PRIVATE_KEY")]
        field: String,
    },

    /// Vaultwarden 의 homelab-env-keys 아이템 필드 값 업데이트.
    /// 예: pxi run vaultwarden env-key-set --value <new_key>
    EnvKeySet {
        #[arg(long, default_value = "/root/.config/vaultwarden-cli")]
        appdata: String,
        #[arg(long, default_value = "DOTENV_PRIVATE_KEY")]
        field: String,
        /// 새 값
        #[arg(long)]
        value: String,
    },

    /// dotenvx 단일 마스터 키 로테이션 원샷:
    ///   1) 새 키페어 생성
    ///   2) rekey.sh 로 모든 .env 재암호화
    ///   3) git commit + push
    ///   4) GitLab 그룹 변수 DOTENV_PRIVATE_KEY 업데이트
    ///   5) Vaultwarden homelab-env-keys 업데이트
    EnvKeyRotate {
        #[arg(long, default_value = "/root/.config/vaultwarden-cli")]
        appdata: String,
        /// 실제 변경 없이 단계만 출력
        #[arg(long)]
        dry_run: bool,
    },
}

fn pct(vmid: u32, args: &[&str]) -> Result<String> {
    let id = vmid.to_string();
    let mut full: Vec<&str> = vec!["exec", &id, "--"];
    full.extend_from_slice(args);
    let out = Command::new("pct")
        .args(&full)
        .output()
        .with_context(|| format!("pct exec {vmid} failed"))?;
    if !out.status.success() {
        return Err(anyhow!(
            "pct {:?} -> {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Status { vmid } => {
            let _ = common::run(
                "pct",
                &[
                    "exec",
                    &vmid.to_string(),
                    "--",
                    "systemctl",
                    "status",
                    "vaultwarden",
                    "--no-pager",
                ],
            );
            Ok(())
        }
        Cmd::Logs { vmid, follow, tail } => {
            let mut args: Vec<String> = vec![
                "exec".into(),
                vmid.to_string(),
                "--".into(),
                "journalctl".into(),
                "-u".into(),
                "vaultwarden".into(),
                "-n".into(),
                tail.to_string(),
                "--no-pager".into(),
            ];
            if follow {
                args.push("-f".into());
            }
            let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let _ = common::run("pct", &refs);
            Ok(())
        }
        Cmd::Restart { vmid } => {
            let _ = common::run(
                "pct",
                &[
                    "exec",
                    &vmid.to_string(),
                    "--",
                    "systemctl",
                    "restart",
                    "vaultwarden",
                ],
            );
            Ok(())
        }
        Cmd::Doctor { vmid } => doctor(vmid),
        Cmd::DomainSet { vmid, url } => domain_set(vmid, &url),
        Cmd::SmtpMailgun {
            vmid,
            host,
            port,
            from,
        } => smtp_mailgun(vmid, &host, port, &from),
        Cmd::BwInstall { rbw, version } => bw_install(rbw, &version),
        Cmd::Backup { vmid } => backup_status(vmid),
        Cmd::InstallSystemd { vmid } => install_systemd(vmid),
        Cmd::InstallBackupTimer { vmid } => install_backup_timer(vmid),
        Cmd::InstallEnv {
            vmid,
            url,
            admin_token,
        } => install_env(vmid, &url, admin_token.as_deref()),
        Cmd::InstallBinary {
            vmid,
            version,
            features,
            force,
        } => install_binary(vmid, version.as_deref(), &features, force),
        Cmd::InstallWebVault {
            vmid,
            version,
            force,
        } => install_web_vault(vmid, version.as_deref(), force),
        Cmd::Bootstrap {
            vmid,
            url,
            admin_token,
            version,
            web_vault_version,
            skip_binary,
        } => bootstrap(
            vmid,
            &url,
            admin_token.as_deref(),
            version.as_deref(),
            web_vault_version.as_deref(),
            skip_binary,
        ),
        Cmd::Upgrade {
            vmid,
            version,
            web_vault_version,
            dry_run,
        } => upgrade(
            vmid,
            version.as_deref(),
            web_vault_version.as_deref(),
            dry_run,
        ),
        Cmd::BwVerify { appdata } => bw_verify(&appdata),
        Cmd::EnvKeyGet { appdata, field } => env_key_get(&appdata, &field),
        Cmd::EnvKeySet {
            appdata,
            field,
            value,
        } => env_key_set(&appdata, &field, &value),
        Cmd::EnvKeyRotate { appdata, dry_run } => env_key_rotate(&appdata, dry_run),
    }
}

fn doctor(vmid: u32) -> Result<()> {
    println!("=== Vaultwarden doctor (LXC {vmid}) ===");
    // 서비스
    let active = pct(vmid, &["systemctl", "is-active", "vaultwarden"]).unwrap_or_default();
    println!("  service: {}", active.trim());

    // config.json 핵심 값
    let domain = pct(
        vmid,
        &[
            "sh",
            "-c",
            "grep -E '\"domain\"' /opt/vaultwarden/data/config.json || true",
        ],
    )
    .unwrap_or_default();
    println!("  domain:  {}", domain.trim());

    let smtp = pct(vmid, &["sh", "-c",
        r#"grep -E '"smtp_(host|port|from)"' /opt/vaultwarden/data/config.json | tr -d ' ,' | tr '\n' ' '"#
    ]).unwrap_or_default();
    println!("  smtp:    {}", smtp.trim());

    // .env 의 ROCKET_TLS 상태
    let tls = pct(
        vmid,
        &[
            "sh",
            "-c",
            r#"grep -E '^ROCKET_TLS|^#\s*ROCKET_TLS' /opt/vaultwarden/.env || echo 'not set'"#,
        ],
    )
    .unwrap_or_default();
    println!("  tls:     {} (Traefik 경유면 off 가 정상)", tls.trim());

    // backup timer
    let timer = pct(
        vmid,
        &[
            "sh",
            "-c",
            r#"systemctl is-active vaultwarden-backup.timer 2>/dev/null || echo missing"#,
        ],
    )
    .unwrap_or_default();
    println!("  backup:  {}", timer.trim());

    Ok(())
}

fn domain_set(vmid: u32, url: &str) -> Result<()> {
    println!("setting domain → {url}");
    let script = format!(
        r#"python3 - <<'PY'
import json
p = "/opt/vaultwarden/data/config.json"
c = json.load(open(p))
c["domain"] = "{url}"
json.dump(c, open(p, "w"), indent=2)
PY
chown vaultwarden:vaultwarden /opt/vaultwarden/data/config.json
systemctl restart vaultwarden
echo ok
"#
    );
    let out = pct(vmid, &["sh", "-c", &script])?;
    println!("{}", out.trim());
    Ok(())
}

fn smtp_mailgun(vmid: u32, host: &str, port: u16, from: &str) -> Result<()> {
    println!("setting smtp → {host}:{port} (mailgun-smtp-proxy) from={from}");
    let script = format!(
        r#"python3 - <<'PY'
import json
p = "/opt/vaultwarden/data/config.json"
c = json.load(open(p))
c["smtp_host"] = "{host}"
c["smtp_port"] = {port}
c["smtp_security"] = "off"
c["smtp_from"] = "{from}"
c["smtp_from_name"] = "Vaultwarden"
c["smtp_username"] = None
c["smtp_password"] = None
c["smtp_auth_mechanism"] = None
c["smtp_accept_invalid_certs"] = True
c["smtp_accept_invalid_hostnames"] = True
json.dump(c, open(p, "w"), indent=2)
PY
# .env 에 auth 변수가 남아 있으면 warning 뜨므로 주석 처리
sed -i 's|^SMTP_USERNAME=|# SMTP_USERNAME=|' /opt/vaultwarden/.env
sed -i 's|^SMTP_PASSWORD=|# SMTP_PASSWORD=|' /opt/vaultwarden/.env
sed -i 's|^SMTP_AUTH_MECHANISM=|# SMTP_AUTH_MECHANISM=|' /opt/vaultwarden/.env
chown vaultwarden:vaultwarden /opt/vaultwarden/data/config.json
systemctl restart vaultwarden
echo ok
"#
    );
    let out = pct(vmid, &["sh", "-c", &script])?;
    println!("{}", out.trim());
    Ok(())
}

fn bw_install(rbw: bool, version: &str) -> Result<()> {
    if rbw {
        println!("installing rbw (Rust bitwarden CLI) via cargo");
        let _ = common::run("cargo", &["install", "rbw", "--locked"]);
    } else {
        println!(
            "installing @bitwarden/cli@{version} (최신 2026.x 은 Vaultwarden 1.34 와 호환 이슈)"
        );
        let pkg = format!("@bitwarden/cli@{version}");
        let _ = common::run("npm", &["install", "-g", &pkg]);
    }
    Ok(())
}

fn backup_status(vmid: u32) -> Result<()> {
    println!("=== backup ===");
    let timer = pct(
        vmid,
        &[
            "systemctl",
            "list-timers",
            "vaultwarden-backup.timer",
            "--all",
            "--no-pager",
        ],
    )
    .unwrap_or_default();
    println!("{}", timer.trim());
    let files = pct(vmid, &["sh", "-c",
        "ls -lah /opt/vaultwarden/data/db.sqlite3.backup.* 2>/dev/null | tail -3 || echo '(no backup files yet)'"
    ]).unwrap_or_default();
    println!("--- recent backup files ---\n{}", files.trim());
    Ok(())
}

// ── install 템플릿 ──

const VAULTWARDEN_UNIT: &str = include_str!("../templates/vaultwarden.service");
const BACKUP_SCRIPT: &str = include_str!("../templates/backup.sh");
const BACKUP_UNIT: &str = include_str!("../templates/vaultwarden-backup.service");
const BACKUP_TIMER: &str = include_str!("../templates/vaultwarden-backup.timer");

fn pct_write(vmid: u32, path: &str, content: &str) -> Result<()> {
    let tmp = format!("/tmp/pxi-vw-push-{}", std::process::id());
    std::fs::write(&tmp, content)?;
    let id = vmid.to_string();
    let out = Command::new("pct")
        .args(["push", &id, &tmp, path])
        .output()?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        return Err(anyhow!(
            "pct push {path}: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

fn install_systemd(vmid: u32) -> Result<()> {
    println!("=== vaultwarden.service 설치 (LXC {vmid}) ===");
    pct_write(
        vmid,
        "/etc/systemd/system/vaultwarden.service",
        VAULTWARDEN_UNIT,
    )?;
    // user + data dir 멱등 보장
    pct(vmid, &["sh", "-c",
        "id -u vaultwarden >/dev/null 2>&1 || useradd --system --no-create-home --shell /bin/false vaultwarden; \
         mkdir -p /opt/vaultwarden/data /opt/vaultwarden/bin; \
         chown -R vaultwarden:vaultwarden /opt/vaultwarden/data"
    ])?;
    pct(vmid, &["systemctl", "daemon-reload"])?;
    pct(vmid, &["systemctl", "enable", "vaultwarden"])?;
    println!(
        "  ok — 바이너리(/opt/vaultwarden/bin/vaultwarden) 배포 후 `systemctl start vaultwarden`"
    );
    Ok(())
}

fn install_backup_timer(vmid: u32) -> Result<()> {
    println!("=== vaultwarden-backup timer 설치 (LXC {vmid}) ===");
    pct_write(vmid, "/opt/vaultwarden/backup.sh", BACKUP_SCRIPT)?;
    pct(vmid, &["chmod", "+x", "/opt/vaultwarden/backup.sh"])?;
    pct(
        vmid,
        &[
            "chown",
            "vaultwarden:vaultwarden",
            "/opt/vaultwarden/backup.sh",
        ],
    )?;
    pct_write(
        vmid,
        "/etc/systemd/system/vaultwarden-backup.service",
        BACKUP_UNIT,
    )?;
    pct_write(
        vmid,
        "/etc/systemd/system/vaultwarden-backup.timer",
        BACKUP_TIMER,
    )?;
    pct(vmid, &["systemctl", "daemon-reload"])?;
    pct(
        vmid,
        &["systemctl", "enable", "--now", "vaultwarden-backup.timer"],
    )?;
    let status = pct(
        vmid,
        &["systemctl", "is-active", "vaultwarden-backup.timer"],
    )
    .unwrap_or_default();
    println!("  timer: {}", status.trim());
    Ok(())
}

fn install_env(vmid: u32, url: &str, admin_token: Option<&str>) -> Result<()> {
    // ADMIN_TOKEN 조회: 인자 → control-plane/.env
    let token = match admin_token {
        Some(t) => t.to_string(),
        None => std::fs::read_to_string("/root/control-plane/.env")
            .ok()
            .and_then(|s| s.lines()
                .map(|l| l.trim_start_matches('#').trim())
                .find_map(|l| l.strip_prefix("VAULTWARDEN_ADMIN_TOKEN=").map(|v| v.trim().to_string()))
                .filter(|v| !v.is_empty()))
            .ok_or_else(|| anyhow!(
                "ADMIN_TOKEN 없음. --admin-token 명시 또는 control-plane/.env 에 VAULTWARDEN_ADMIN_TOKEN= 추가"))?,
    };

    let env_content = format!(
        "# Generated by `pxi run vaultwarden install-env`\n\
         ADMIN_TOKEN={token}\n\
         ROCKET_ADDRESS=0.0.0.0\n\
         # ROCKET_TLS 는 Traefik 경유라 off. 다시 쓰려면 주석 해제.\n\
         DATA_FOLDER=/opt/vaultwarden/data\n\
         DATABASE_MAX_CONNS=10\n\
         WEB_VAULT_FOLDER=/opt/vaultwarden/web-vault\n\
         WEB_VAULT_ENABLED=true\n\
         # 도메인 — invite/verify 메일 링크에 사용 (config.json 이 있으면 그쪽 우선)\n\
         DOMAIN={url}\n\
         # SMTP — mailgun-smtp-proxy shim 경유 (CF same-zone 수신자도 배달)\n\
         SMTP_HOST={MAILGUN_SHIM_HOST}\n\
         SMTP_PORT=2526\n\
         SMTP_SECURITY=off\n\
         SMTP_FROM=devops@ranode.net\n\
         SMTP_FROM_NAME=Vaultwarden\n"
    );
    pct_write(vmid, "/opt/vaultwarden/.env", &env_content)?;
    pct(
        vmid,
        &["chown", "vaultwarden:vaultwarden", "/opt/vaultwarden/.env"],
    )?;
    pct(vmid, &["chmod", "640", "/opt/vaultwarden/.env"])?;
    println!("  ok — /opt/vaultwarden/.env (DOMAIN={url}, SMTP mailgun-shim)");
    Ok(())
}

// ── install-binary / install-web-vault / bootstrap ──

const VW_UPSTREAM: &str = "https://github.com/dani-garcia/vaultwarden.git";
const WEB_VAULT_RELEASE_BASE: &str =
    "https://github.com/dani-garcia/bw_web_builds/releases/download";

fn install_binary(vmid: u32, version: Option<&str>, features: &str, force: bool) -> Result<()> {
    // 이미 있고 force 아니면 skip
    let exists = pct(
        vmid,
        &[
            "sh",
            "-c",
            "test -x /opt/vaultwarden/bin/vaultwarden && echo yes || echo no",
        ],
    )
    .unwrap_or_default();
    if exists.trim() == "yes" && !force {
        println!("  /opt/vaultwarden/bin/vaultwarden 이미 존재 — skip (--force 로 재빌드)");
        return Ok(());
    }

    // 태그 결정: 지정 or latest
    let tag = match version {
        Some(v) => v.to_string(),
        None => resolve_latest("dani-garcia/vaultwarden")?,
    };
    println!("=== vaultwarden {tag} 빌드 (LXC {vmid}, features={features}) ===");
    println!("  rustup + apt deps + git clone + cargo build (수분 소요)");

    // apt deps + rustup + build 를 한 스크립트에
    let script = format!(
        r#"set -e
apt-get update -qq
apt-get install -y --no-install-recommends \
    git curl ca-certificates build-essential pkg-config \
    libssl-dev libsqlite3-dev libpq-dev libmariadb-dev-compat >/dev/null

# rustup (per-run, minimal)
if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null
fi
. "$HOME/.cargo/env"

SRC=/opt/vaultwarden/src
mkdir -p /opt/vaultwarden/bin /opt/vaultwarden/data

# shallow clone + tag 체크아웃이 기존 .git 에서 꼬이면 --version metadata 가
# 예전 값 embed 되므로, 매번 src 를 새로 clone 하는 편이 안전.
rm -rf "$SRC"
git clone --depth 1 --branch {tag} {upstream} "$SRC"

cd "$SRC"
HEAD_TAG=$(git describe --tags --always)
echo "src HEAD = $HEAD_TAG"
cargo build --release --features {features} --no-default-features
install -m 0755 target/release/vaultwarden /opt/vaultwarden/bin/vaultwarden
chown -R vaultwarden:vaultwarden /opt/vaultwarden/bin /opt/vaultwarden/data
echo "binary_sha=$(sha256sum /opt/vaultwarden/bin/vaultwarden | awk '{{print $1}}')"
"#,
        tag = tag,
        upstream = VW_UPSTREAM,
        features = features
    );
    let out = pct(vmid, &["sh", "-c", &script])?;
    print!("{out}");
    Ok(())
}

fn install_web_vault(vmid: u32, version: Option<&str>, force: bool) -> Result<()> {
    let exists = pct(
        vmid,
        &[
            "sh",
            "-c",
            "test -f /opt/vaultwarden/web-vault/index.html && echo yes || echo no",
        ],
    )
    .unwrap_or_default();
    if exists.trim() == "yes" && !force {
        println!("  /opt/vaultwarden/web-vault 이미 존재 — skip (--force 로 재설치)");
        return Ok(());
    }
    let tag = match version {
        Some(v) => v.to_string(),
        None => resolve_latest("dani-garcia/bw_web_builds")?,
    };
    println!("=== web-vault {tag} 배치 (LXC {vmid}) ===");
    let url = format!("{WEB_VAULT_RELEASE_BASE}/{tag}/bw_web_{tag}.tar.gz");
    let script = format!(
        r#"set -e
apt-get install -y --no-install-recommends curl ca-certificates tar >/dev/null
mkdir -p /opt/vaultwarden
rm -rf /opt/vaultwarden/web-vault.new
mkdir -p /opt/vaultwarden/web-vault.new
curl -fsSL "{url}" | tar -xz -C /opt/vaultwarden/web-vault.new --strip-components=1
[ -f /opt/vaultwarden/web-vault.new/index.html ] || {{ echo "web-vault tarball missing index.html"; exit 1; }}
rm -rf /opt/vaultwarden/web-vault
mv /opt/vaultwarden/web-vault.new /opt/vaultwarden/web-vault
chown -R vaultwarden:vaultwarden /opt/vaultwarden/web-vault
echo ok
"#,
        url = url
    );
    let out = pct(vmid, &["sh", "-c", &script])?;
    print!("{out}");
    Ok(())
}

fn bootstrap(
    vmid: u32,
    url: &str,
    admin_token: Option<&str>,
    vw_version: Option<&str>,
    web_vault_version: Option<&str>,
    skip_binary: bool,
) -> Result<()> {
    println!("=== Vaultwarden bootstrap (LXC {vmid}) ===");
    install_env(vmid, url, admin_token)?;
    install_systemd(vmid)?;
    install_backup_timer(vmid)?;
    install_web_vault(vmid, web_vault_version, false)?;
    if !skip_binary {
        install_binary(vmid, vw_version, "sqlite", false)?;
    } else {
        println!("  --skip-binary 지정, 바이너리 설치 skip");
    }
    // start
    pct(vmid, &["systemctl", "restart", "vaultwarden"])?;
    let st = pct(vmid, &["systemctl", "is-active", "vaultwarden"]).unwrap_or_default();
    println!("  service: {}", st.trim());
    Ok(())
}

/// GitHub API 로 owner/repo 의 latest release tag 조회.
fn resolve_latest(repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let out = Command::new("curl")
        .args(["-sSL", "-H", "Accept: application/vnd.github+json", &url])
        .output()?;
    if !out.status.success() {
        return Err(anyhow!(
            "latest release 조회 실패: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let body = String::from_utf8_lossy(&out.stdout);
    // GitHub pretty-prints JSON → `"tag_name": "v..."` (공백/탭 허용). 가볍게 파싱.
    let key_idx = body.find("\"tag_name\"").ok_or_else(|| {
        anyhow!(
            "tag_name 파싱 실패: {}",
            &body.chars().take(160).collect::<String>()
        )
    })?;
    let after_colon = body[key_idx..]
        .find(':')
        .map(|p| key_idx + p + 1)
        .ok_or_else(|| anyhow!("tag_name 콜론 없음"))?;
    let rest = body[after_colon..].trim_start();
    let quote_open = rest
        .find('"')
        .ok_or_else(|| anyhow!("tag_name 값 시작 따옴표 없음"))?;
    let after_quote = &rest[quote_open + 1..];
    let quote_close = after_quote
        .find('"')
        .ok_or_else(|| anyhow!("tag_name 끝 따옴표 찾기 실패"))?;
    Ok(after_quote[..quote_close].to_string())
}

// ── upgrade ──

fn upgrade(vmid: u32, target: Option<&str>, web_target: Option<&str>, dry_run: bool) -> Result<()> {
    let current_vw = pct(
        vmid,
        &[
            "sh",
            "-c",
            "/opt/vaultwarden/bin/vaultwarden --version 2>/dev/null | head -1 || echo unknown",
        ],
    )
    .unwrap_or_default();
    let latest_vw = resolve_latest("dani-garcia/vaultwarden").unwrap_or_else(|_| "?".into());
    let latest_web = resolve_latest("dani-garcia/bw_web_builds").unwrap_or_else(|_| "?".into());

    let vw_tag = target.map(str::to_string).unwrap_or(latest_vw.clone());
    let web_tag = web_target.map(str::to_string).unwrap_or(latest_web.clone());

    println!("=== Vaultwarden upgrade plan (LXC {vmid}) ===");
    println!("  current  : {}", current_vw.trim());
    println!("  target   : vaultwarden {vw_tag} / web-vault {web_tag}");

    if dry_run {
        println!("  dry-run — 실제 변경 없음");
        return Ok(());
    }

    // 1) DB snapshot (backup.sh 가 있으면 그걸 호출, 없으면 직접 sqlite backup)
    println!("\n[1/4] db snapshot…");
    let snap = pct(
        vmid,
        &[
            "sh",
            "-c",
            r#"
if [ -x /opt/vaultwarden/backup.sh ]; then
    sudo -u vaultwarden /opt/vaultwarden/backup.sh 2>&1 | tail -1
else
    TS=$(date +%Y%m%d%H%M%S)
    sqlite3 /opt/vaultwarden/data/db.sqlite3 ".backup /opt/vaultwarden/data/db.sqlite3.backup.$TS"
    chown vaultwarden:vaultwarden /opt/vaultwarden/data/db.sqlite3.backup.$TS
    echo "snapshot db.sqlite3.backup.$TS"
fi
"#,
        ],
    )?;
    println!("  {}", snap.trim());

    // 2) binary rebuild
    println!("\n[2/4] binary rebuild (수분 소요)…");
    install_binary(vmid, Some(&vw_tag), "sqlite", true)?;

    // 3) web-vault
    println!("\n[3/4] web-vault…");
    install_web_vault(vmid, Some(&web_tag), true)?;

    // 4) restart + verify
    println!("\n[4/4] restart + verify…");
    pct(vmid, &["systemctl", "restart", "vaultwarden"])?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    let active = pct(vmid, &["systemctl", "is-active", "vaultwarden"]).unwrap_or_default();
    let new_ver = pct(
        vmid,
        &[
            "sh",
            "-c",
            "/opt/vaultwarden/bin/vaultwarden --version 2>/dev/null | head -1 || echo unknown",
        ],
    )
    .unwrap_or_default();
    println!("  service: {} / version: {}", active.trim(), new_ver.trim());
    Ok(())
}

// ── bw-verify ──

fn read_env(key: &str) -> Option<String> {
    let content = std::fs::read_to_string("/root/control-plane/.env").ok()?;
    content
        .lines()
        .map(|l| l.trim_start_matches('#').trim())
        .find_map(|l| {
            l.strip_prefix(&format!("{key}="))
                .map(|v| v.trim().trim_matches('"').to_string())
        })
        .filter(|v| !v.is_empty())
}

fn bw_cmd(appdata: &str, password: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .env("BW_PASSWORD", password)
        .args(args)
        .output()
}

fn bw_verify(appdata: &str) -> Result<()> {
    let url = read_env("VAULTWARDEN_URL")
        .ok_or_else(|| anyhow!("VAULTWARDEN_URL 가 control-plane/.env 에 없음"))?;
    let email = read_env("VAULTWARDEN_EMAIL")
        .ok_or_else(|| anyhow!("VAULTWARDEN_EMAIL 가 control-plane/.env 에 없음"))?;
    let password = read_env("VAULTWARDEN_MASTER_PASSWORD")
        .ok_or_else(|| anyhow!("VAULTWARDEN_MASTER_PASSWORD 가 control-plane/.env 에 없음"))?;

    println!("=== bw-verify ({email}) ===");
    std::fs::create_dir_all(appdata)?;

    // bw --version
    let ver = Command::new("bw").arg("--version").output()?;
    let ver_s = String::from_utf8_lossy(&ver.stdout);
    println!(
        "  bw     : {}",
        ver_s.trim().split('\n').next().unwrap_or("?")
    );

    // config server
    let _ = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .args(["config", "server", &url])
        .output()?;

    // 혹시 이전 세션 남아있으면 logout (조용히)
    let _ = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .arg("logout")
        .output();

    // login (raw: session key or 에러)
    let login = bw_cmd(
        appdata,
        &password,
        &["login", &email, "--passwordenv", "BW_PASSWORD", "--raw"],
    )?;
    let login_out = String::from_utf8_lossy(&login.stdout);
    let login_err = String::from_utf8_lossy(&login.stderr);
    let session: String = login_out
        .lines()
        .chain(login_err.lines())
        .rev()
        .find(|l| {
            let t = l.trim();
            t.len() >= 40
                && t.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+/=".contains(c))
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if session.is_empty() {
        println!("  login  : ❌");
        let msg = login_err
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("(no stderr)");
        println!("           → {}", msg.trim());
        return Err(anyhow!("bw login 실패"));
    }
    println!("  login  : ✅ (session {} chars)", session.len());

    // sync
    let sync = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .env("BW_SESSION", &session)
        .args(["sync", "--session", &session])
        .output()?;
    println!(
        "  sync   : {}",
        String::from_utf8_lossy(&sync.stdout).trim()
    );

    // list items
    let list = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .args(["list", "items", "--session", &session])
        .output()?;
    let items_json = String::from_utf8_lossy(&list.stdout);
    let count = items_json.matches("\"id\":").count();
    println!("  items  : {count}");

    Ok(())
}

// ── homelab-env-keys 공통 헬퍼 ──────────────────────────────────────────────

const ENV_KEYS_ITEM_NAME: &str = "homelab-env-keys";

/// bw 언락 세션 반환 (login --apikey + unlock 조합)
fn bw_session(appdata: &str) -> Result<String> {
    let url = read_env("VAULTWARDEN_URL")
        .ok_or_else(|| anyhow!("VAULTWARDEN_URL 가 control-plane/.env 에 없음"))?;
    let email = read_env("VAULTWARDEN_EMAIL")
        .ok_or_else(|| anyhow!("VAULTWARDEN_EMAIL 가 control-plane/.env 에 없음"))?;
    let password = read_env("VAULTWARDEN_MASTER_PASSWORD")
        .ok_or_else(|| anyhow!("VAULTWARDEN_MASTER_PASSWORD 가 control-plane/.env 에 없음"))?;
    let client_id = read_env("VAULTWARDEN_CLIENT_ID")
        .ok_or_else(|| anyhow!("VAULTWARDEN_CLIENT_ID 가 control-plane/.env 에 없음"))?;
    let client_secret = read_env("VAULTWARDEN_CLIENT_SECRET")
        .ok_or_else(|| anyhow!("VAULTWARDEN_CLIENT_SECRET 가 control-plane/.env 에 없음"))?;

    std::fs::create_dir_all(appdata)?;

    // config server (idempotent)
    let _ = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .args(["config", "server", &url])
        .output()?;

    // login --apikey (이미 로그인돼 있으면 무시)
    let _ = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .env("BW_CLIENTID", &client_id)
        .env("BW_CLIENTSECRET", &client_secret)
        .args(["login", "--apikey"])
        .output()?;

    // unlock → raw session key
    let unlock = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .env("BW_PASSWORD", &password)
        .args(["unlock", "--passwordenv", "BW_PASSWORD", "--raw"])
        .output()?;

    let session = String::from_utf8_lossy(&unlock.stdout).trim().to_string();
    if session.is_empty() {
        let err = String::from_utf8_lossy(&unlock.stderr);
        return Err(anyhow!("bw unlock 실패: {}", err.trim()));
    }
    Ok(session)
}

/// 아이템 JSON 에서 특정 필드 값 추출
fn extract_field(item_json: &str, field_name: &str) -> Option<String> {
    // 간단한 파싱: "fields" 배열에서 name 매칭
    let needle = format!("\"name\":\"{field_name}\"");
    let pos = item_json.find(&needle)?;
    // "value":"..." 를 needle 뒤에서 찾기
    let after = &item_json[pos..];
    let val_start = after.find("\"value\":\"")?  + "\"value\":\"".len();
    let val_end = after[val_start..].find('"')? + val_start;
    Some(after[val_start..val_end].to_string())
}

/// bw get item (by name) → JSON string
fn bw_get_item(appdata: &str, session: &str, name: &str) -> Result<String> {
    // sync 먼저
    let _ = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .args(["sync", "--session", session])
        .output()?;

    let out = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .args(["get", "item", name, "--session", session])
        .output()?;

    let json = String::from_utf8_lossy(&out.stdout).to_string();
    if json.trim().is_empty() || json.contains("\"message\":") {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("bw get item '{}' 실패: {}", name, err.trim()));
    }
    Ok(json)
}

// ── env-key-get ─────────────────────────────────────────────────────────────

fn env_key_get(appdata: &str, field: &str) -> Result<()> {
    let session = bw_session(appdata)?;
    let json = bw_get_item(appdata, &session, ENV_KEYS_ITEM_NAME)?;
    let value = extract_field(&json, field)
        .ok_or_else(|| anyhow!("필드 '{field}' 를 {ENV_KEYS_ITEM_NAME} 에서 찾을 수 없음"))?;
    // stdout 에만 값 출력 (스크립트 캡처용)
    print!("{value}");
    Ok(())
}

// ── env-key-set ─────────────────────────────────────────────────────────────

fn env_key_set(appdata: &str, field: &str, value: &str) -> Result<()> {
    let session = bw_session(appdata)?;
    let json = bw_get_item(appdata, &session, ENV_KEYS_ITEM_NAME)?;

    // Python 으로 JSON 조작 (serde_json 없이)
    let updated = Command::new("python3")
        .args([
            "-c",
            &format!(
                r#"
import json, sys
d = json.loads(sys.argv[1])
found = False
for f in d.get('fields', []):
    if f['name'] == sys.argv[2]:
        f['value'] = sys.argv[3]
        found = True
if not found:
    d.setdefault('fields', []).append({{'name': sys.argv[2], 'value': sys.argv[3], 'type': 1}})
print(json.dumps(d))
"#
            ),
            &json,
            field,
            value,
        ])
        .output()?;

    let updated_json = String::from_utf8_lossy(&updated.stdout).to_string();
    if updated_json.trim().is_empty() {
        return Err(anyhow!("JSON 업데이트 실패"));
    }

    // bw encode | bw edit item <id>
    let item_id = {
        let id_out = Command::new("python3")
            .args(["-c", "import json,sys; print(json.loads(sys.argv[1])['id'])", &json])
            .output()?;
        String::from_utf8_lossy(&id_out.stdout).trim().to_string()
    };

    let encode = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .args(["encode"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    use std::io::Write;
    let mut encode = encode;
    encode.stdin.as_mut().unwrap().write_all(updated_json.as_bytes())?;
    let encoded = encode.wait_with_output()?;
    let encoded_str = String::from_utf8_lossy(&encoded.stdout).to_string();

    let edit = Command::new("bw")
        .env("BITWARDENCLI_APPDATA_DIR", appdata)
        .args(["edit", "item", &item_id, "--session", &session])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let mut edit = edit;
    edit.stdin.as_mut().unwrap().write_all(encoded_str.trim().as_bytes())?;
    let result = edit.wait_with_output()?;

    if result.status.success() {
        eprintln!("✓ {ENV_KEYS_ITEM_NAME} [{field}] 업데이트 완료");
    } else {
        let err = String::from_utf8_lossy(&result.stderr);
        return Err(anyhow!("bw edit 실패: {}", err.trim()));
    }
    Ok(())
}

// ── env-key-rotate ───────────────────────────────────────────────────────────

fn env_key_rotate(appdata: &str, dry_run: bool) -> Result<()> {
    println!("=== dotenvx 마스터 키 로테이션 ===");

    let repo_root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    let root = String::from_utf8_lossy(&repo_root.stdout).trim().to_string();
    if root.is_empty() {
        return Err(anyhow!("git 저장소 루트를 찾을 수 없음"));
    }

    // 1. 새 키페어 생성 (임시 .env 에 dotenvx encrypt 적용)
    println!("\n[1/5] 새 키페어 생성");
    let tmp_dir = std::env::temp_dir().join(format!("pxi-rekey-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir)?;
    let tmp_env = tmp_dir.join(".env");
    std::fs::write(&tmp_env, "PLACEHOLDER=value\n")?;

    let gen = Command::new("dotenvx")
        .arg("encrypt")
        .arg("-f")
        .arg(&tmp_env)
        .output()?;
    if !gen.status.success() {
        return Err(anyhow!("dotenvx encrypt 실패: {}", String::from_utf8_lossy(&gen.stderr)));
    }

    let env_content = std::fs::read_to_string(&tmp_env)?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let new_private_key = env_content
        .lines()
        .find_map(|l| l.strip_prefix("DOTENV_PRIVATE_KEY=").or_else(|| l.strip_prefix("DOTENV_PRIVATE_KEY=\"").map(|v| v.trim_end_matches('"'))))
        .map(|v| v.trim_matches('"').to_string())
        .ok_or_else(|| anyhow!("새 DOTENV_PRIVATE_KEY 추출 실패"))?;
    let new_public_key = env_content
        .lines()
        .find_map(|l| l.strip_prefix("DOTENV_PUBLIC_KEY=").or_else(|| l.strip_prefix("DOTENV_PUBLIC_KEY=\"").map(|v| v.trim_end_matches('"'))))
        .map(|v| v.trim_matches('"').to_string())
        .ok_or_else(|| anyhow!("새 DOTENV_PUBLIC_KEY 추출 실패"))?;

    println!("  공개키: {}", &new_public_key[..16]);
    println!("  비밀키: {}... ({}자)", &new_private_key[..8], new_private_key.len());

    if dry_run {
        println!("\n[dry-run] 이후 단계 생략");
        println!("  DOTENV_PRIVATE_KEY={new_private_key}");
        println!("  DOTENV_PUBLIC_KEY={new_public_key}");
        return Ok(());
    }

    // 2. rekey.sh 실행
    println!("\n[2/5] rekey.sh 실행");
    let rekey = Command::new("bash")
        .arg(format!("{root}/control-plane/scripts/rekey.sh"))
        .env("MASTER_PRIVATE_KEY", &new_private_key)
        .current_dir(&root)
        .status()?;
    if !rekey.success() {
        return Err(anyhow!("rekey.sh 실패"));
    }

    // 3. git commit + push
    println!("\n[3/5] git commit + push");
    Command::new("git")
        .args(["add", "control-plane/envs/"])
        .current_dir(&root)
        .status()?;
    let msg = format!("chore: dotenvx 마스터 키 로테이션 {}", chrono_now());
    Command::new("git")
        .args(["commit", "-m", &msg])
        .current_dir(&root)
        .status()?;
    Command::new("git")
        .args(["push"])
        .current_dir(&root)
        .status()?;
    println!("  ✓ push 완료");

    // 4. GitLab 그룹 변수 업데이트
    println!("\n[4/5] GitLab DOTENV_PRIVATE_KEY 업데이트");
    let gl_token = read_env("GITLAB_TOKEN")
        .or_else(|| read_env("GITLAB_API_TOKEN"))
        .ok_or_else(|| anyhow!("GITLAB_TOKEN 또는 GITLAB_API_TOKEN 이 control-plane/.env 에 없음"))?;
    let gl_url = read_env("GITLAB_URL").unwrap_or_else(|| "http://10.0.50.63".to_string()); // LINT_ALLOW: GITLAB_URL 미설정 시 fallback — control-plane/.env 에 항상 있어야 함
    let group_id = "283";

    // PUT (이미 존재)
    let resp = Command::new("curl")
        .args([
            "-s", "-X", "PUT",
            &format!("{gl_url}/api/v4/groups/{group_id}/variables/DOTENV_PRIVATE_KEY"),
            "-H", &format!("PRIVATE-TOKEN: {gl_token}"),
            "-H", "Content-Type: application/json",
            "-d", &format!(r#"{{"key":"DOTENV_PRIVATE_KEY","value":"{new_private_key}","masked":true,"protected":false}}"#),
        ])
        .output()?;
    let resp_str = String::from_utf8_lossy(&resp.stdout);
    if resp_str.contains("\"key\":\"DOTENV_PRIVATE_KEY\"") {
        println!("  ✓ GitLab 변수 업데이트 완료");
    } else {
        eprintln!("  WARN: GitLab 응답 확인 필요: {}", resp_str.trim());
    }

    // 5. Vaultwarden 업데이트
    println!("\n[5/5] Vaultwarden homelab-env-keys 업데이트");
    env_key_set(appdata, "DOTENV_PRIVATE_KEY", &new_private_key)?;
    env_key_set(appdata, "DOTENV_PUBLIC_KEY", &new_public_key)?;

    println!("\n✅ 키 로테이션 완료");
    println!("  새 공개키: {new_public_key}");
    Ok(())
}

fn chrono_now() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown-date".to_string())
}
