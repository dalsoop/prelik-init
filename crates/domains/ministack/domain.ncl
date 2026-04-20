let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "ministack",
  description = "MiniStack (LocalStack AWS 에뮬레이터)",
  tags = { product = 'tool, layer = 'remote, platform = 'proxmox },
  provides = [
    "ministack install",
    "ministack uninstall",
    "ministack start",
    "ministack stop",
    "ministack restart",
    "ministack status",
    "ministack reset",
    "ministack logs",
    "ministack update",
    "ministack doctor"
  ],
} | Domain
