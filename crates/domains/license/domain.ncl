let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "license",
  description = "라이선스 관리 (Keygen CE)",
  tags = { product = 'service, layer = 'remote, platform = 'proxmox },
  provides = [
    "license activate",
    "license status",
    "license deactivate",
    "license check-in",
    "license doctor",
    "license self-update"
  ],
} | Domain
