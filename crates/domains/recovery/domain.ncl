let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "recovery",
  description = "LXC config 스냅샷 + audit log",
  tags = { product = 'infra, layer = 'host, platform = 'proxmox },
  provides = [
    "recovery doctor",
    "recovery create",
    "recovery list",
    "recovery restore",
    "recovery delete",
    "recovery audit-log",
    "recovery audit-show"
  ],
} | Domain
