{
  name = "deploy",
  description = "서비스 레시피 기반 LXC 자동 배포",
  tags = { product = 'infra, layer = 'remote, platform = 'proxmox },
  requires = ["lxc"],
  provides = [
    "deploy service",
    "deploy list-recipes",
  ],
}
