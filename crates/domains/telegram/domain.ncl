let { Domain } = import "../../../ncl/domain_contract.ncl" in

{
  name = "telegram",
  description = "Telegram 봇 관리",
  tags = { product = 'service, layer = 'tool, platform = 'any },
  provides = [
    "telegram register",
    "telegram send",
    "telegram send-photo",
    "telegram list",
    "telegram remove",
    "telegram verify",
    "telegram status",
    "telegram bots",
    "telegram webhook",
    "telegram pairing-approve",
    "telegram generate",
    "telegram setup-all",
    "telegram assign",
    "telegram channels",
    "telegram bot-register",
    "telegram bot-rename",
    "telegram doctor"
  ],
} | Domain
