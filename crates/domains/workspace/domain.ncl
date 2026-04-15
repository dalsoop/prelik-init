{
  name = "workspace",
  description = "작업 환경 설정 (tmux + shell alias + fzf/bat 등)",
  tags = { product = 'init, layer = 'tool, platform = 'debian },
  provides = [
    "workspace tmux-setup",
    "workspace shell-setup",
    "workspace status",
  ],
}
