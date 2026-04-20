let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "comfyui",
  description = "ComfyUI LXC 설치 관리",
  tags = { product = 'ai, layer = 'remote, platform = 'proxmox },
  requires = ["lxc"],
  provides = [
    "comfyui gpu-passthrough",
    "comfyui install",
    "comfyui status",
    "comfyui doctor"
  ],
} | Domain
