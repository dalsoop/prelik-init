# pxi 도메인 contract — crates/domains/*/domain.ncl 전부 이 스키마 준수.
#
# 사용:
#   let { Domain } = import "../../../ncl/domain_contract.ncl" in
#   {
#     name = "lxc",
#     ...
#   } | Domain

let Product = std.contract.from_predicate (fun v =>
  std.array.any ((==) v) [
    'infra, 'desktop, 'ai, 'devops, 'service, 'monitor, 'tool, 'media, 'network
  ]
) in

let Layer = std.contract.from_predicate (fun v =>
  std.array.any ((==) v) [
    'host, 'remote, 'tool
  ]
) in

let Platform = std.contract.from_predicate (fun v =>
  std.array.any ((==) v) [
    'proxmox, 'linux, 'any
  ]
) in

# name 관례: 소문자 + 하이픈. pxi run <name> 에서 노출되는 이름.
let NameStr = std.contract.from_predicate (fun v =>
  std.is_string v
  && std.string.length v > 0
  && std.string.is_match "^[a-z][a-z0-9-]*$" v
) in

# provides 엔트리 형식: "<domain> <subcommand>" 문자열.
let ProvidesEntry = std.contract.from_predicate (fun v =>
  std.is_string v
  && std.string.length v > 0
) in

{
  Domain = {
    name | NameStr,
    description | String,
    tags | {
      product | Product,
      layer | Layer,
      platform | Platform,
    },
    # 이 도메인이 선행 설치를 요구하는 다른 도메인 이름 목록.
    requires | Array NameStr | optional,
    # 이 도메인이 pxi CLI 로 노출하는 "<name> <subcommand>" 엔트리 목록.
    provides | Array ProvidesEntry,
  },
}
