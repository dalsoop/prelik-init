let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "traefik",
  description = "Traefik 리버스 프록시 관리",
  tags = { product = 'service, layer = 'host, platform = 'proxmox },
  requires = ["lxc"],
  provides = [
    "traefik setup",
    "traefik recreate",
    "traefik list",
    "traefik add",
    "traefik remove",
    "traefik resync",
    "traefik drift",
    "traefik cert-verify",
    "traefik cert-recheck",
    "traefik cloudflare-sync",
    "traefik doctor"
  ],
} | Domain
