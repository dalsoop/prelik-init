//! 도메인 레지스트리 — ncl/domains.ncl → locale.json → runtime.
//!
//! 로드 tier (우선순위):
//!   1) `paths::locale_json()` 파일시스템 — install-local.sh / release tarball 이 배치
//!   2) 빌드 시 embed 된 locale.json — build.rs 가 `nickel export` 로 생성
//!   3) hard-fail
//!
//! Tier 1 은 사용자가 수정할 수 있는 경로, tier 2 는 바이너리에 구워진 보증.
//! 둘 다 동일한 `SUPPORTED_FORMAT_VERSION` 검사 대상.
//!
//! Runtime 에 nickel CLI 의존성 없음.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Registry 가 이해하는 locale.json 포맷 버전. 여기를 벗어나면 load() 가 hard-fail.
/// 새 버전 추가 시 `match` 암(arm) 로 graceful migration 작성.
const SUPPORTED_FORMAT_VERSION: u32 = 1;

/// build.rs 가 `nickel export ncl/domains.ncl` 결과를 OUT_DIR/locale.json 으로 기록.
/// nickel 미설치 빌드 환경에서는 empty JSON(`{}`) 이 기록되고, `from_str` 파싱이
/// format_version 검사에서 실패 → tier 3 hard-fail.
const EMBEDDED_LOCALE_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/locale.json"));

#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    pub format_version: u32,
    pub domains: BTreeMap<String, Domain>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Domain {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub tags: Tags,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
    /// SSOT contract 에는 없음. runtime 이 "미구현" 구분에만 사용.
    /// locale.json 에 `"enabled": false` 를 실으면 `planned()` 쪽으로 분류.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Tags {
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
}

fn default_true() -> bool { true }

impl Registry {
    pub fn load() -> anyhow::Result<Self> {
        // Tier 1: 파일시스템 locale.json
        if let Ok(path) = crate::paths::locale_json() {
            if path.exists() {
                let raw = std::fs::read_to_string(&path)
                    .map_err(|e| anyhow::anyhow!("{} 읽기 실패: {e}", path.display()))?;
                return Self::parse_with_version(&raw, &path.display().to_string());
            }
        }
        // Tier 2: 바이너리에 embed 된 locale.json (build.rs 가 생성).
        // nickel 미설치 빌드면 빈 `{}` 가 embed 되어 여기서 format_version 검사 실패 →
        // tier 3 hard-fail 로 진행.
        if let Ok(reg) = Self::parse_with_version(EMBEDDED_LOCALE_JSON, "<embedded>") {
            return Ok(reg);
        }
        // Tier 3: hard-fail
        anyhow::bail!(
            "locale.json 이 없음. fresh clone 이면 다음을 먼저 실행:\n  \
             scripts/install-local.sh\n\
             릴리스 tarball 사용 시 install.sh 가 자동 배치해야 함. \
             소스에서 빌드했지만 nickel 이 미설치면 embedded locale 도 없음 — \
             'pxi run bootstrap nickel' 후 재빌드."
        );
    }

    fn parse_with_version(raw: &str, source: &str) -> anyhow::Result<Self> {
        let reg: Registry = serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("{} JSON 파싱 실패: {e}", source))?;
        if reg.format_version != SUPPORTED_FORMAT_VERSION {
            anyhow::bail!(
                "{} format_version={} 은 runtime 이 지원하지 않음 (supported={}). \
                 Nickel SSOT 업그레이드 또는 pxi 바이너리 업그레이드 필요.",
                source,
                reg.format_version,
                SUPPORTED_FORMAT_VERSION
            );
        }
        Ok(reg)
    }

    pub fn available(&self) -> Vec<&Domain> {
        let mut list: Vec<&Domain> = self.domains.values().filter(|d| d.enabled).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn planned(&self) -> Vec<&Domain> {
        let mut list: Vec<&Domain> = self.domains.values().filter(|d| !d.enabled).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }
}

// 레거시 호환 — 기존 호출부(`known_domains`, `binary_name`) 유지.
// 새 코드는 Registry::load() 를 우선 사용.
pub fn known_domains() -> Vec<(&'static str, &'static str)> {
    vec![
        ("code-server", "code-server (VS Code 웹) 설치/제거"),
        ("wordpress", "WordPress LXC 설치/설정/관리"),
    ]
}

pub fn binary_name(domain: &str) -> String {
    crate::brand::domain_bin(domain)
}
