let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "host",
  description = "호스트 시스템 관리",
  tags = { product = 'infra, layer = 'host, platform = 'proxmox },
  provides = [
    "host bootstrap",
    "host status",
    "host monitor",
    "host ssh-keygen",
    "host smb-open",
    "host smb-close",
    "host gh-auth",
    "host self-update",
    "host postfix-relay",
    "host doctor"
  ],
} | Domain
