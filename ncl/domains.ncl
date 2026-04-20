# pxi 도메인 SSOT 인덱스 — 실제 도메인 정의는 crates/domains/*/domain.ncl.
# 이 파일은 단순히 모든 도메인 instance 를 import 해 하나의 레코드로 노출.
#
# 각 도메인 파일은 `ncl/domain_contract.ncl` 의 Domain contract 를 준수하므로,
# 이 파일을 eval 하는 것만으로 29개 전부 검증됨. Nickel 이 import 시점에 계약 적용.

{
  account     = import "../crates/domains/account/domain.ncl",
  ai          = import "../crates/domains/ai/domain.ncl",
  backup      = import "../crates/domains/backup/domain.ncl",
  bootstrap   = import "../crates/domains/bootstrap/domain.ncl",
  cloudflare  = import "../crates/domains/cloudflare/domain.ncl",
  code-server = import "../crates/domains/code-server/domain.ncl",
  comfyui     = import "../crates/domains/comfyui/domain.ncl",
  connect     = import "../crates/domains/connect/domain.ncl",
  deploy      = import "../crates/domains/deploy/domain.ncl",
  elk         = import "../crates/domains/elk/domain.ncl",
  host        = import "../crates/domains/host/domain.ncl",
  infisical   = import "../crates/domains/infisical/domain.ncl",
  iso         = import "../crates/domains/iso/domain.ncl",
  license     = import "../crates/domains/license/domain.ncl",
  lxc         = import "../crates/domains/lxc/domain.ncl",
  mail        = import "../crates/domains/mail/domain.ncl",
  ministack   = import "../crates/domains/ministack/domain.ncl",
  monitor     = import "../crates/domains/monitor/domain.ncl",
  nas         = import "../crates/domains/nas/domain.ncl",
  net         = import "../crates/domains/net/domain.ncl",
  node        = import "../crates/domains/node/domain.ncl",
  recovery    = import "../crates/domains/recovery/domain.ncl",
  service     = import "../crates/domains/service/domain.ncl",
  telegram    = import "../crates/domains/telegram/domain.ncl",
  traefik     = import "../crates/domains/traefik/domain.ncl",
  vm          = import "../crates/domains/vm/domain.ncl",
  wordpress   = import "../crates/domains/wordpress/domain.ncl",
  workspace   = import "../crates/domains/workspace/domain.ncl",
  xdesktop    = import "../crates/domains/xdesktop/domain.ncl",
}
