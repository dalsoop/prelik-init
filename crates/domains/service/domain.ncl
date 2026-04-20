let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "service",
  description = "서비스 레지스트리 (파일 기반)",
  tags = { product = 'infra, layer = 'host, platform = 'proxmox },
  provides = [
    "service list",
    "service sync",
    "service add",
    "service remove",
    "service diff",
    "service info",
    "service doctor",
    "service move"
  ],
} | Domain
