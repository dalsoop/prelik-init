//! 서비스 alias → VMID 레지스트리 로더.
//!
//! 정본 TOML: `/root/control-plane/config/services_registry.toml` 또는
//! `/etc/pxi/services_registry.toml`. 두 곳 없으면 내장 기본값 사용.
//!
//! Nickel contract: `control-plane/nickel/services_registry_contract.ncl`.
//! 런타임 (여기) 에선 TOML 파싱만 하고 contract 검증은 CI 로 밀어둠 —
//! control-plane 의 `validate-all.sh` 가 Nickel 엔진으로 schema 체크.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceEntry {
    pub vmid: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServicesRegistry {
    pub services: HashMap<String, ServiceEntry>,
}

impl ServicesRegistry {
    /// 정본 경로 우선순위:
    ///   1) `/root/control-plane/config/services_registry.toml` (개발/운영 호스트)
    ///   2) `/etc/pxi/services_registry.toml` (설치 후 시스템)
    /// 둘 다 없으면 Err.
    pub fn load() -> anyhow::Result<Self> {
        for path in [
            PathBuf::from("/root/control-plane/config/services_registry.toml"),
            PathBuf::from("/etc/pxi/services_registry.toml"),
        ] {
            if path.exists() {
                let raw = std::fs::read_to_string(&path)?;
                return Ok(toml::from_str(&raw)?);
            }
        }
        anyhow::bail!(
            "services_registry.toml 못 찾음 — \
             /root/control-plane/config/ 또는 /etc/pxi/ 에 배치"
        )
    }

    /// alias 로 VMID 조회. 없으면 Err (도메인은 bail 으로 처리).
    pub fn vmid_for(&self, alias: &str) -> anyhow::Result<&str> {
        self.services
            .get(alias)
            .map(|e| e.vmid.as_str())
            .ok_or_else(|| anyhow::anyhow!(
                "services_registry: alias {alias} 미등록. \
                 /root/control-plane/config/services_registry.toml 에 추가 필요."
            ))
    }

    /// alias 로 canonical IP 유도 (vmid 조회 후 `convention::canonical_ip`).
    pub fn canonical_ip(&self, alias: &str) -> anyhow::Result<String> {
        let vmid = self.vmid_for(alias)?;
        crate::convention::canonical_ip(vmid)
    }
}

/// 편의 API — 대부분의 도메인은 `vmid_for("mail")` 한 번만 부르면 됨.
pub fn vmid_for(alias: &str) -> anyhow::Result<String> {
    let reg = ServicesRegistry::load()?;
    reg.vmid_for(alias).map(|s| s.to_string())
}

/// 편의 API — alias 로 바로 canonical IP.
pub fn canonical_ip_for(alias: &str) -> anyhow::Result<String> {
    let reg = ServicesRegistry::load()?;
    reg.canonical_ip(alias)
}
