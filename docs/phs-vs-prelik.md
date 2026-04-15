# phs ↔ prelik 비교표

prelik-init은 dalsoop의 내부 도구 **phs (proxmox-host-setup)**에서 공유 가치 있는
부분만 추출해 독립 프로젝트로 분리한 것입니다. 이 문서는 **실제 동작 차이**를
솔직하게 기록합니다.

## 스코프 비교

| 도메인 | phs 커맨드 수 | prelik 이식 | 상태 |
|--------|-------------|-------------|------|
| lxc | 14 | 10 | 대부분 (부트스트랩 옵션 누락) |
| traefik | 10 | 4 | 핵심만 (원격 노드 지원 없음) |
| mail | 12 | 3 | 부분 (mailpit + postfix-relay만) |
| cloudflare | 16 | 6 | 부분 (DNS CRUD + Worker — SSL/Pages 없음) |
| ai | 41 | 5 | 극히 일부 (플러그인 설치 중심) |
| connect | — | 4 | 신규 (phs의 config에 해당) |
| bootstrap | — | 1 도메인 | 신규 (의존성 설치 통합) |
| **host** | 8 | **0** | **미이식** |
| **nas** | 10 | **0** | **미이식** |
| **telegram** | 14 | **0** | **미이식** |
| **workspace** | 12 | **0** | **미이식** |
| **account** | 4 | **0** | **미이식** |

## 커맨드 대응표

### lxc

| phs | prelik | 동등성 |
|-----|--------|--------|
| `phs infra lxc-create --vmid X --hostname Y [--bootstrap]` | `prelik run lxc create --vmid X --hostname Y --ip Z` | **부분** — prelik은 `--ip` 필수, `--bootstrap` 없음 |
| `phs infra lxc-delete --vmid X` | `prelik run lxc delete X --force` | 동등 (prelik은 --force 필수) |
| `phs infra lxc-start --vmid X` | `prelik run lxc start X` | 동등 |
| `phs infra lxc-stop --vmid X` | `prelik run lxc stop X` | 동등 |
| `phs infra lxc-reboot --vmid X` | `prelik run lxc restart X` | 동등 |
| `phs infra lxc-enter --vmid X` | `prelik run lxc enter X` | 동등 |
| `phs infra lxc-list` | `prelik run lxc list` | 동등 |
| `phs infra lxc-config --vmid X` | **없음** | 미이식 — `pct config X`로 대체 |
| `phs infra lxc-mount` | **없음** | 미이식 |
| `phs infra lxc-resize` | **없음** | 미이식 |
| `phs infra lxc-bootstrap` | **없음** | 미이식 (tmux/셸 설정) |
| `phs infra lxc-align-vmid-ip` | **없음** | 미이식 (VMID 재배치) |
| `phs infra backup --vmid X` | `prelik run lxc backup X` | 동등 |

### traefik

| phs | prelik | 동등성 |
|-----|--------|--------|
| `phs infra traefik-add --name --domain --backend [--node]` | `prelik run traefik route-add --vmid --name --domain --backend [--use-cf]` | **부분** — prelik은 `--vmid` 필수, `--node` 미지원 |
| `phs infra traefik-remove` | `prelik run traefik route-remove` | 동등 |
| `phs infra traefik-list` | `prelik run traefik route-list` | 동등 |
| `phs infra traefik-recreate` | `prelik run traefik recreate` | 동등 (CF env 자동 주입 동일) |
| `phs infra traefik-setup` | **없음** | 미이식 (LXC 생성까지 통합) |
| `phs infra traefik-drift` | **없음** | 미이식 |
| `phs infra traefik-resync` | **없음** | 미이식 |
| `phs infra traefik-cloudflare-sync` | **recreate에 통합됨** | 동등 |
| `phs infra traefik-cert-verify` | **없음** | 미이식 |
| `phs infra traefik-cert-recheck` | **없음** | 미이식 |

### cloudflare

| phs | prelik | 동등성 |
|-----|--------|--------|
| `phs cloudflare dns records` | `prelik run cloudflare dns-list` | 동등 |
| `phs cloudflare dns add` | `prelik run cloudflare dns-add [--audience]` | **prelik이 우수** (audience 기반 proxied 자동) |
| `phs cloudflare dns update` | `prelik run cloudflare dns-update` | 동등 |
| `phs cloudflare dns delete` | `prelik run cloudflare dns-delete` | 동등 |
| `phs cloudflare email status` | **없음** | 미이식 |
| `phs cloudflare email forward` | **없음** | 미이식 |
| `phs cloudflare email worker-attach-all` | `prelik run cloudflare email-worker-attach-all [--dry-run]` | **prelik이 우수** (--dry-run) |
| `phs cloudflare ssl issue` | **없음** | 미이식 |
| `phs cloudflare pages deploy` | **없음** | 미이식 |

### mail

| phs | prelik | 동등성 |
|-----|--------|--------|
| `phs host postfix-relay` | `prelik run mail postfix-relay` | **prelik이 우수** (자동 백업, 전체 롤백) |
| `phs infra mail-setup` | **없음** | 미이식 (Maddy 풀 스택 설치) |
| `phs infra mail-status` | `prelik run mail status` | 부분 |
| (Mailpit 설치) | `prelik run mail install-mailpit` | **prelik 전용** |

### ai

| phs | prelik | 동등성 |
|-----|--------|--------|
| `phs ai install` | `prelik run ai install` | 동등 |
| `phs ai octopus-install` | `prelik run ai octopus-install` | 동등 |
| `phs ai superpowers-install` | `prelik run ai superpowers-install` | 동등 |
| `phs ai codex-plugin-install --fork` | `prelik run ai codex-plugin-install --fork` | 동등 |
| `phs ai adversarial-review-hook` | `prelik run ai adversarial-review-hook` | **prelik이 우수** (기존 훅 보존) |
| `phs ai comfyui-*` (10개) | **없음** | 미이식 (ComfyUI 워크플로우) |
| `phs ai openclaw-*` (12개) | **없음** | 미이식 (OpenClaw 게이트웨이) |
| `phs ai codex-review-bot`, `mount`, `unmount`, `credential-sync` 등 | **없음** | 미이식 |

## "prelik이 우수한 점"

phs 대비 prelik이 **개선한** 부분:

1. **audience 기반 proxied 자동** — `--audience kr/global/internal`로 실수 방지
2. **--dry-run** — 파괴적 CF 작업 미리보기
3. **완전한 postfix-relay 롤백** — main.cf + sasl_passwd + sender_canonical 일괄 백업/복원
4. **기존 Stop 훅 보존** — marker 기반 append/filter로 사용자 다른 훅 파괴 안 함
5. **mktemp + chmod 600 + RAII 가드** — secrets을 /tmp에 평문 노출 안 함
6. **install flock** — 동시 설치 race 차단
7. **도구 단위 install/remove** — `--only rust,nickel`로 선택
8. **Nickel SSOT** — 도메인 메타데이터 ncl 파일 단일 소스

## "prelik이 못 하는 점"

phs 대비 prelik이 **못 하는** 주요 기능:

1. **원격 노드 관리** — phs는 `--node ranode-3960x` 옵션, prelik은 로컬만
2. **LXC 부트스트랩** — phs의 `--bootstrap`이 자동 tmux/셸/veil 설치
3. **NAS/Synology/TrueNAS** — phs는 마운트/공유 관리, prelik 없음
4. **Telegram 봇** — phs는 8개 봇 통합 관리, prelik 없음
5. **Workspace 설정** — tmux/셸 환경 일괄, prelik 없음
6. **RBAC/roles.toml** — phs는 dalroot 역할 기반, prelik 없음
7. **ComfyUI/OpenClaw** — phs의 대형 AI 도메인

## 결론

**prelik-init은 phs의 "공유 가치 있는 ~25%"만 추출한 서브셋**입니다.
개인 사용자가 새 Proxmox/LXC 환경에서 빠르게 시작할 수 있는 기본기는
갖추었지만, dalsoop의 실제 운영에 필요한 모든 기능은 없습니다.

향후 로드맵:
- `--node` 원격 지원 (traefik, lxc)
- `lxc create --bootstrap` 이식
- `cloudflare` SSL/Pages/email forward
- `nas`, `host` 도메인 신규

자세한 내용은 [CHANGELOG.md](../CHANGELOG.md) 참조.
