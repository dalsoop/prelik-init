let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "connect",
  description = "외부 서비스 연결 관리 (.env + dotenvx)",
  tags = { product = 'tool, layer = 'host, platform = 'any },
  provides = [
    "connect set",
    "connect remove",
    "connect list",
    "connect encrypt",
    "connect doctor"
  ],
} | Domain
