# pxi-init 도메인 레지스트리

let Domain = {
  name | String,
  description | String,
  enabled | Bool | default = true,
  binary | String,
  subcommands | Array String | default = [],
}
in

{
  domains = {
    code-server = {
      name = "code-server",
      description = "code-server (VS Code 웹) 설치/제거 + Traefik + git auto-sync",
      enabled = true,
      binary = "pxi-code-server",
      subcommands = ["install", "remove", "doctor"],
    } | Domain,

    wordpress = {
      name = "wordpress",
      description = "WordPress LXC 설치/설정/관리",
      enabled = true,
      binary = "pxi-wordpress",
      subcommands = ["install", "setup", "cli", "delete", "doctor"],
    } | Domain,
  },
}
