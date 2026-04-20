let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "deploy",
  description = "레시피 기반 LXC 자동 배포",
  tags = { product = 'devops, layer = 'remote, platform = 'proxmox },
  requires = ["lxc"],
  provides = [
    "deploy service",
    "deploy list-recipes",
    "deploy doctor",
    "deploy ao-setup",
    "deploy homelable-domain",
    "deploy homelable-seed",
    "deploy homelable-sync-run",
    "deploy homelable-sync-enable",
    "deploy homelable-sync-disable",
    "deploy homelable-sync-status",
    "deploy infra-control-domain",
    "deploy infra-control-page-refresh",
    "deploy formbricks-build",
    "deploy formbricks-form",
    "deploy omarchy-setup",
    "deploy omarchy-iso",
    "deploy domain-map-set",
    "deploy domain-map-remove",
    "deploy domain-map-list",
    "deploy mail-setup"
  ],
} | Domain
