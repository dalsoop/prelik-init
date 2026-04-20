//! 브랜드 상수 — 프로젝트 전체 이름/경로의 유일한 원천.
//!
//! 이름을 바꾸고 싶으면 이 파일만 수정하고 `cargo build`.
//! 시스템 경로 마이그레이션은 `pxi rebrand` 커맨드가 처리.

/// CLI 짧은 이름 (바이너리명, 커맨드)
pub const SHORT: &str = "pxi";

/// 프로젝트 전체 이름
pub const FULL: &str = "proxmox-init";

/// GitHub org/repo
pub const REPO: &str = "dalsoop/proxmox-init";

/// 설정 디렉토리 이름 (/etc/{NAME} 또는 ~/.config/{NAME})
pub const CONFIG_DIR_NAME: &str = "pxi";

/// 데이터 디렉토리 이름 (/var/lib/{NAME})
pub const DATA_DIR_NAME: &str = "pxi";

/// 도메인 바이너리 prefix (e.g. "pxi-elk", "pxi-telegram")
pub const BIN_PREFIX: &str = "pxi";

/// 도메인 실행 시 표시명
pub const fn bin_name(domain: &str) -> String {
    // const fn에서 String 못 만들므로 런타임 헬퍼로
    unreachable!()
}

/// 도메인 바이너리 이름 생성
pub fn domain_bin(domain: &str) -> String {
    format!("{}-{}", BIN_PREFIX, domain)
}

/// 시스템 경로들
pub mod paths {
    use std::path::PathBuf;

    pub fn config_root() -> &'static str {
        concat!("/etc/", "pxi")
    }

    pub fn data_root() -> &'static str {
        concat!("/var/lib/", "pxi")
    }

    /// 설정 디렉토리 경로 (XDG_CONFIG_HOME 또는 /etc/pxi 폴백).
    pub fn config_dir() -> anyhow::Result<PathBuf> {
        // XDG_CONFIG_HOME → ~/.config/pxi. 없으면 /etc/pxi.
        if let Ok(home) = std::env::var("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(home).join("pxi"));
        }
        if let Ok(home) = std::env::var("HOME") {
            let user_cfg = PathBuf::from(home).join(".config").join("pxi");
            if user_cfg.exists() {
                return Ok(user_cfg);
            }
        }
        Ok(PathBuf::from(config_root()))
    }
}
