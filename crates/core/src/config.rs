//! 런타임 설정 로더. ~/.config/pxi/config.toml 또는 /etc/pxi/config.toml.

use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub proxmox: ProxmoxConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub lxc: LxcConfig,
}

/// LXC 생성 기본값. `pxi run lxc create` 가 default_value 하드코딩 대신 여기서 로드.
/// 도메인별 override 가 필요한 쪽 (xdesktop 무거움, wordpress MariaDB 등) 은 자기 기본값 유지.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LxcConfig {
    #[serde(default = "default_cores")]
    pub cores: String,
    #[serde(default = "default_memory")]
    pub memory: String,
    #[serde(default = "default_disk")]
    pub disk: String,
    #[serde(default = "default_template")]
    pub template: String,
    #[serde(default = "default_storage")]
    pub storage: String,
    #[serde(default = "default_bridge")]
    pub bridge: String,
}

impl Default for LxcConfig {
    fn default() -> Self {
        Self {
            cores: default_cores(),
            memory: default_memory(),
            disk: default_disk(),
            template: default_template(),
            storage: default_storage(),
            bridge: default_bridge(),
        }
    }
}

fn default_cores() -> String { "2".into() }
fn default_memory() -> String { "2048".into() }
fn default_disk() -> String { "8".into() }
fn default_template() -> String { "debian-13".into() }
fn default_storage() -> String { "local-lvm".into() }
fn default_bridge() -> String { "vmbr1".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxmoxConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub node: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default)]
    pub bridge: String,
    #[serde(default)]
    pub gateway: String,
    #[serde(default = "default_subnet")]
    pub subnet: u8,
}

fn default_subnet() -> u8 { 16 }

impl Config {
    /// 로드 규약 (services 레지스트리와 동일):
    ///   - 파일 없음 → Self::default() (fresh install 안전망)
    ///   - 파일 존재하지만 읽기/파싱 실패 → **bail** (관리자 override 무시되는 silent fallback 방지)
    pub fn load() -> anyhow::Result<Self> {
        let path = paths::config_dir()?.join("config.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("{} 읽기 실패: {e}", path.display()))?;
        let cfg = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("{} TOML 파싱 실패: {e}", path.display()))?;
        Ok(cfg)
    }
}
