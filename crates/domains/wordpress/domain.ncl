let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "wordpress",
  description = "WordPress LXC 설치/설정/관리",
  tags = { product = 'service, layer = 'remote, platform = 'proxmox },
  provides = [
    "wordpress install",
    "wordpress setup",
    "wordpress cli",
    "wordpress delete",
    "wordpress doctor",
    "wordpress lockdown-apply",
    "wordpress lockdown-status",
    "wordpress lockdown-remove",
    "wordpress lockdown-test"
  ],
} | Domain
