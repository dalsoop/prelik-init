let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "iso",
  description = "Proxmox ISO 스토리지 관리",
  tags = { product = 'infra, layer = 'host, platform = 'proxmox },
  provides = [
    "iso doctor",
    "iso list",
    "iso storage-add-nfs",
    "iso storage-add-cifs",
    "iso storage-remove",
    "iso download",
    "iso remove"
  ],
} | Domain
