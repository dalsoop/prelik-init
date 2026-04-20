let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "elk",
  description = "ELK 스택 관리 (ES + Kibana + Logstash)",
  tags = { product = 'service, layer = 'remote, platform = 'proxmox },
  provides = [
    "elk status",
    "elk open",
    "elk connect",
    "elk disconnect",
    "elk search",
    "elk indices",
    "elk cleanup",
    "elk restart",
    "elk install",
    "elk doctor"
  ],
} | Domain
