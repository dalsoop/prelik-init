let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "bootstrap",
  description = "의존성 개별/일괄 설치·제거 (apt/rust/gh/dotenvx/nickel)",
  tags = { product = 'infra, layer = 'host, platform = 'linux },
  provides = [
    "bootstrap install",
    "bootstrap remove",
    "bootstrap doctor",
    "bootstrap list",
    "bootstrap manifest"
  ],
} | Domain
