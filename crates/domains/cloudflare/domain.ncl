let { Domain } = import "../../../ncl/contracts/domain.ncl" in

{
  name = "cloudflare",
  description = "Cloudflare DNS + Email Routing + SSL + Pages",
  tags = { product = 'service, layer = 'tool, platform = 'any },
  provides = [
    "cloudflare dns-add",
    "cloudflare dns-list",
    "cloudflare dns-update",
    "cloudflare dns-upsert",
    "cloudflare dns-delete",
    "cloudflare zones",
    "cloudflare email-status",
    "cloudflare email-forward",
    "cloudflare email-forward-all",
    "cloudflare email-worker-attach",
    "cloudflare email-worker-attach-all",
    "cloudflare ssl-install",
    "cloudflare ssl-issue",
    "cloudflare ssl-renew",
    "cloudflare ssl-list",
    "cloudflare ssl-status",
    "cloudflare pages-list",
    "cloudflare pages-create",
    "cloudflare pages-domain",
    "cloudflare pages-delete",
    "cloudflare pages-deploy",
    "cloudflare doctor"
  ],
} | Domain
