let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "nas",
  description = "NAS 마운트 + Synology/TrueNAS 관리",
  tags = { product = 'service, layer = 'remote, platform = 'proxmox },
  provides = [
    "nas mount",
    "nas unmount",
    "nas list",
    "nas status",
    "nas shares",
    "nas pools",
    "nas share-create",
    "nas share-delete",
    "nas synology-list",
    "nas synology-sync",
    "nas synology-link",
    "nas doctor"
  ],
} | Domain
