let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "node",
  description = "Proxmox 클러스터 노드 관리",
  tags = { product = 'infra, layer = 'host, platform = 'proxmox },
  provides = [
    "node doctor",
    "node list",
    "node info",
    "node exec",
    "node join",
    "node remove",
    "node sync"
  ],
} | Domain
