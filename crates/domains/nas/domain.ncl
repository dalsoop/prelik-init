{
  name = "nas",
  description = "NAS 마운트 관리 (SMB/CIFS + NFS, Synology/TrueNAS 범용)",
  tags = { product = 'init, layer = 'platform, platform = 'debian },
  provides = [
    "nas mount",
    "nas unmount",
    "nas list",
    "nas persist",
  ],
}
