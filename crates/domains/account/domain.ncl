let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "account",
  description = "리눅스 계정 관리",
  tags = { product = 'service, layer = 'host, platform = 'linux },
  provides = [
    "account create",
    "account remove",
    "account teardown",
    "account list",
    "account status",
    "account ssh-key-add",
    "account roles-init",
    "account roles-apply",
    "account roles-status",
    "account proxmox-silo",
    "account doctor"
  ],
} | Domain
