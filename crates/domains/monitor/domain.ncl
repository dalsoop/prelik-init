let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "monitor",
  description = "호스트/LXC/VM 리소스 모니터링",
  tags = { product = 'monitor, layer = 'host, platform = 'any },
  provides = [
    "monitor doctor",
    "monitor host",
    "monitor lxc",
    "monitor vm",
    "monitor all",
    "monitor global-verify",
    "monitor health-check",
    "monitor healthcheck",
    "monitor audit-log",
    "monitor kuma-sync-run"
  ],
} | Domain
