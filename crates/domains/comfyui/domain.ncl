{
  name = "comfyui",
  description = "ComfyUI LXC 설치 관리 (GPU 패스스루 + 설치 + systemd)",
  tags = { product = 'ai, layer = 'remote, platform = 'proxmox },
  requires = ["lxc"],
  provides = [
    "comfyui install",
    "comfyui gpu-passthrough",
    "comfyui status",
  ],
}
