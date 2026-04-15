# 도메인 프리셋 — 자주 쓰는 조합을 이름으로 묶음
{
  presets = {
    web = {
      description = "웹 호스팅 기본 세트",
      domains = ["bootstrap", "lxc", "traefik", "cloudflare"],
    },
    mail = {
      description = "메일 발송/수신 아카이브 스택",
      domains = ["bootstrap", "lxc", "mail", "cloudflare", "connect"],
    },
    minimal = {
      description = "필수 최소 (bootstrap만)",
      domains = ["bootstrap"],
    },
    dev = {
      description = "개발자 도구 (AI 플러그인 중심)",
      domains = ["bootstrap", "ai", "connect"],
    },
  }
}
