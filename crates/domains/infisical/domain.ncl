let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "infisical",
  description = "Infisical (시크릿 관리 플랫폼)",
  tags = { product = 'tool, layer = 'remote, platform = 'proxmox },
  provides = [
    "infisical install",
    "infisical uninstall",
    "infisical start",
    "infisical stop",
    "infisical restart",
    "infisical status",
    "infisical logs",
    "infisical update",
    "infisical doctor",
    "infisical smtp"
  ],
} | Domain
