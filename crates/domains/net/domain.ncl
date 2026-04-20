let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "net",
  description = "네트워크 진단",
  tags = { product = 'network, layer = 'host, platform = 'proxmox },
  provides = [
    "net doctor",
    "net interfaces",
    "net routes",
    "net bridges",
    "net dns",
    "net ping",
    "net net-status",
    "net net-audit",
    "net net-fix",
    "net ip-audit",
    "net ip-fix",
    "net ingress-audit",
    "net ingress-fix"
  ],
} | Domain
