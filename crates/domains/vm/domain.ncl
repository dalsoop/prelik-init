let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "vm",
  description = "Proxmox QEMU VM 관리",
  tags = { product = 'infra, layer = 'remote, platform = 'proxmox },
  provides = [
    "vm list",
    "vm status",
    "vm start",
    "vm stop",
    "vm reboot",
    "vm delete",
    "vm backup",
    "vm resize",
    "vm console",
    "vm doctor",
    "vm setup",
    "vm korean",
    "vm clipboard",
    "vm type"
  ],
} | Domain
