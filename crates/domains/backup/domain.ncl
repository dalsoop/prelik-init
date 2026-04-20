let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "backup",
  description = "LXC/VM 백업 (vzdump)",
  tags = { product = 'tool, layer = 'host, platform = 'proxmox },
  provides = [
    "backup now",
    "backup list",
    "backup schedule-add",
    "backup schedule-list",
    "backup schedule-remove",
    "backup restore",
    "backup doctor"
  ],
} | Domain
