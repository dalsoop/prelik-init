let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "code-server",
  description = "code-server (VS Code 웹) 설치/제거",
  tags = { product = 'devops, layer = 'remote, platform = 'proxmox },
  provides = [
    
  ],
} | Domain
