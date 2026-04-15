{
  name = "telegram",
  description = "Telegram 봇 관리 (알림 발송, 봇 토큰 저장)",
  tags = { product = 'auto, layer = 'tool, platform = 'any },
  provides = [
    "telegram register",
    "telegram send",
    "telegram list",
    "telegram remove",
  ],
}
