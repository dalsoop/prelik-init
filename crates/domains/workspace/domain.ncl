let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "workspace",
  description = "작업 환경 (tmux + shell 도구 + LXC 환경)",
  tags = { product = 'devops, layer = 'remote, platform = 'proxmox },
  provides = [
    "workspace tmux-setup",
    "workspace tmux-start",
    "workspace shell-setup",
    "workspace nvim-markdown",
    "workspace lxc-shell",
    "workspace lxc-tmux",
    "workspace status",
    "workspace doctor"
  ],
} | Domain
