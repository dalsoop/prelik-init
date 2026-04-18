# proxmox-init (pxi)

[![Version](https://img.shields.io/github/v/release/dalsoop/proxmox-init)](https://github.com/dalsoop/proxmox-init/releases)
[![Domains](https://img.shields.io/badge/domains-28-blueviolet)]()
[![Commands](https://img.shields.io/badge/commands-309-blue)]()
[![License](https://img.shields.io/badge/license-MIT-lightgrey)]()

> Proxmox/LXC/Debian 서버용 **도메인 기반 설치형 CLI**.
> 28개 도메인이 독립 바이너리로 배포됩니다. 필요한 것만 설치.

```bash
curl -fsSL https://install.prelik.com | bash
pxi init
```

## 사용법

```bash
pxi install elk telegram wordpress    # 도메인 설치
pxi run elk status                    # 도메인 실행
pxi run telegram send --bot ops --chat 123 --text "배포 완료"
pxi list                              # 설치된 도메인
pxi available                         # 사용 가능한 도메인
pxi doctor                            # 전체 상태 점검
```

## 도메인 (28개)

| 도메인 | 설명 |
|---|---|
| `account` | 리눅스 계정 + Proxmox RBAC (roles, proxmox-silo) |
| `ai` | Claude/Codex CLI, mount/perm-max, OpenClaw, ComfyUI |
| `backup` | vzdump 기반 LXC/VM 백업 + 스케줄 |
| `bootstrap` | apt/rust/gh/dotenvx 의존성 설치 |
| `cloudflare` | DNS / Email Routing / SSL / Pages |
| `comfyui` | ComfyUI LXC 설치 (GPU 패스스루) |
| `connect` | 외부 서비스 연결 (.env + dotenvx) |
| `deploy` | 레시피 기반 LXC 배포 (Homelable, Formbricks 등) |
| `elk` | ELK 스택 (Elasticsearch + Kibana + Logstash) |
| `host` | 호스트 bootstrap, monitor, postfix-relay |
| `infisical` | Infisical 시크릿 플랫폼 |
| `iso` | Proxmox ISO 스토리지 관리 |
| `license` | Keygen CE 라이선스 관리 |
| `lxc` | LXC lifecycle + bootstrap + route-audit |
| `mail` | Maddy + Postfix relay |
| `ministack` | LocalStack AWS 에뮬레이터 |
| `monitor` | 리소스 모니터링 + health-check |
| `nas` | NAS 마운트 + Synology/TrueNAS API |
| `net` | 네트워크 진단 (audit, fix, ingress) |
| `node` | Proxmox 클러스터 노드 관리 |
| `recovery` | LXC config 스냅샷/복원 |
| `service` | 파일 기반 서비스 레지스트리 |
| `telegram` | 텔레그램 봇 (send, webhook, generate) |
| `traefik` | Traefik 리버스 프록시 관리 |
| `vm` | Proxmox VM lifecycle |
| `wordpress` | WordPress LXC 배포 |
| `workspace` | tmux + shell + nvim |

## 프리셋

```bash
pxi install --preset web      # bootstrap, lxc, traefik, cloudflare
pxi install --preset mail     # bootstrap, lxc, mail, cloudflare, connect
pxi install --preset dev      # bootstrap, ai, connect
```

## 서비스 레지스트리

```bash
pxi run service list                    # 도메인별 서비스 목록
pxi run service add --domain prelik.com --name blog --host blog.prelik.com --ip 10.0.50.200 --port 80
pxi run service sync                    # Traefik 자동 동기화
```

## 이름 변경

```bash
pxi rebrand newname --apply    # 바이너리 + 경로 일괄 변경
./scripts/rebrand.sh pxi newname && cargo build --release  # 소스 전체
```

## 라이선스

MIT
