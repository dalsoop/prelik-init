let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "mail",
  description = "Maddy + Mailpit + Postfix relay + CF Email Sending proxy",
  tags = { product = 'service, layer = 'remote, platform = 'proxmox },
  requires = ["lxc"],
  provides = [
    "mail install-mailpit",
    "mail postfix-relay",
    "mail status",
    "mail doctor",
    "mail setup",
    "mail cf-proxy-install",
    "mail cf-proxy-quota",
    "mail cf-proxy-sync"
  ],
} | Domain
