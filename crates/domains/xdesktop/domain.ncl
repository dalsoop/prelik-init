let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "xdesktop",
  description = "X11 원격 데스크톱 LXC 설치 관리 (Xpra + 한글 + Helium)",
  tags = { product = 'desktop, layer = 'remote, platform = 'proxmox },
  requires = ["lxc"],
  provides = [
    "xdesktop setup",
    "xdesktop install",
    "xdesktop expose",
    "xdesktop status",
    "xdesktop destroy",
    "xdesktop verify",
    "xdesktop dev",
    "xdesktop audit-all",
    "xdesktop audit-install-timer",
    "xdesktop audit-uninstall-timer",
    "xdesktop doctor"
  ],
} | Domain
