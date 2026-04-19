# pxi-xdesktop

X11 원격 데스크톱 LXC — 브라우저 하나로 접속하는 풀 한글 데스크톱 환경.

## 스택

- **원격 표시**: Xpra `start-desktop` + HTML5 내장 클라이언트 (인증 없음)
- **HTTP 프록시**: nginx HTTP/1.1 bridge (Xpra HTTP/1.0 → 클라이언트 HTTP/1.1 변환 + keepalive pool)
- **데스크톱**: XFCE4 + xfce4-goodies
- **로케일**: `ko_KR.UTF-8` + Noto CJK / Nanum 폰트
- **입력기**: `fcitx5` + `fcitx5-hangul` (Chromium 계열 호환)
- **브라우저**: Helium (ungoogled-chromium 포크), `--no-sandbox` 래퍼
- **인증**: 없음 (50.internal.kr 내부망 규약)

## 서브커맨드

```
pxi run xdesktop setup    --vmid N --hostname H --ip CIDR [--host DOMAIN]
pxi run xdesktop install  --vmid N                   # 기존 LXC에 데스크톱만
pxi run xdesktop expose   --vmid N --host DOMAIN     # traefik 라우트만
pxi run xdesktop dev      --vmid N [--repos a/b,c/d] # 개발 도구 + GitHub 키
pxi run xdesktop status   --vmid N                   # LXC/xpra/nginx/패치 상태
pxi run xdesktop verify   --vmid N --host DOMAIN     # 스모크 테스트 12개
pxi run xdesktop destroy  --vmid N --yes             # LXC + traefik route 제거
pxi run xdesktop doctor                              # pct/pxi-lxc/pxi-service 확인
```

## 전형적 워크플로

```bash
# 1. 전체 배포
pxi run xdesktop setup \
  --vmid 50210 \
  --hostname xdesktop-01 \
  --ip 10.0.50.210/16 \
  --host xdesktop.50.internal.kr

# 2. 검증
pxi run xdesktop verify --vmid 50210 --host xdesktop.50.internal.kr

# 3. 개발 도구 추가 (선택)
pxi run xdesktop dev --vmid 50210 \
  --github-user dalsoop \
  --repos "dalsoop/proxmox-init,imputnet/helium-linux"

# 4. 접속
# 브라우저에서 https://xdesktop.50.internal.kr/ 열기
```

## 삽질 기록 — Xpra HTML5 의 함정들

### 1. Safari 502 burst (PR #18)
Xpra Python HTTP 서버가 **HTTP/1.0** 으로 응답 → 응답 후 backend conn close.
Safari HTTP/2 multistream 동시 8+ 요청 시 traefik 이 RST 받음.

**Fix**: LXC 내부에 nginx HTTP/1.1 bridge 삽입.
`외부 :14500 ← nginx → 127.0.0.1:14501 (Xpra)` keepalive pool 32.

### 2. 드래그 좌표 offset (PR #24, #26)
`start-desktop` 모드에서도 Xpra HTML5 가 루트창 위에 자체 타이틀바
(`<div id="head1" class="windowhead">`) 렌더. `update_offsets()` 가
`$(header).css("height")` 를 `topoffset` 에 더해 canvas 좌표가 밀림.

**Fix**: `.windowhead { display: none; height: 0 }` — jQuery `.css("height")` 가
`"0px"` 반환 → topoffset=0. JS 패치도 시도했으나 cursor 렌더링 부작용으로 롤백.

### 3. 커서 안 보임 (PR #22)
Xvfb 는 기본 커서 없음. xsettings `CursorThemeName` 이 `empty` 로 비어있어
어디서 가져와야 할지 몰라 공백.

**Fix**: `dmz-cursor-theme` 설치 + xsettings.xml 에 `CursorThemeName=Adwaita`,
size 24 + Xresources 백업.

### 4. 빈 사각형 4개 (PR #19)
기본 XFCE panel 이 **pager** (4 워크스페이스) 를 상단에 렌더 → 회색 빈 칸.

**Fix**: 커스텀 panel XML 주입. pager/actions 제거, workspace 1개 고정.

### 5. 하단 dock 기어 아이콘 (PR #19)
기본 XFCE 런처가 `exo-open --launch ...` 경유 생성 ID 로 .desktop 참조
→ 아이콘 resolve 실패 폴백(기어).

**Fix**: XFCE 4.20+ 의 `items = [xfce4-terminal.desktop, ...]` 시스템
.desktop 직접 참조 사용.

### 6. 압축본 stale (PR #26)
nginx `gzip_static` 이 `.gz`/`.br` 우선 서빙 → CSS 수정해도 Safari 가 구
압축본 받음.

**Fix**: 설치 시 `.gz`/`.br` 제거.

### 7. apt upgrade 회귀 (PR #26)
xpra-html5 패키지 업그레이드하면 CSS 수정분 조용히 덮어씀.

**Fix**: `dpkg-divert --rename --add` 로 원본을 `.distrib` 로 밀어두고
우리 버전 유지. 이후 apt 는 `.distrib` 만 건드림.

### 8. HTML5 connect 화면 password 필드 (PR #7)
서버는 `--tcp-auth` 없이 뜨는데 연결 폼에 password 칸이 상시 노출 → UX 혼란.

**Fix**: `index.html` 을 `/connect.html?autoconnect=true` 즉시 리다이렉트 페이지로 교체.

### 9. Xvfb 해상도 8192x4096 (PR #20)
Xpra 기본 Xvfb 화면 크기가 8192x4096 → HTML5 클라이언트에서 비현실적.

**Fix**: `--resize-display=1920x1080` 추가. 클라이언트 접속 시 뷰포트에 맞춰 조정.

## 파일 위치

| | 경로 |
|---|---|
| 소스 (pve 호스트) | `/opt/proxmox-init/crates/domains/xdesktop/` |
| 바이너리 | `/usr/local/bin/pxi-xdesktop` |
| 설치 본체 (embedded) | `scripts/install-desktop.sh` |
| 개발환경 (embedded) | `scripts/dev-setup.sh` |
| LXC 내부 xpra unit | `/etc/systemd/system/xpra-xdesktop.service` |
| LXC 내부 nginx site | `/etc/nginx/sites-enabled/xdesktop` |
| LXC 내부 xpra backup | `/usr/share/xpra/www/{css,js}/*.distrib` (dpkg-divert) |
| traefik route | `50100:/opt/traefik/dynamic/xdesktop-{vmid}.yml` |

## 생존 조건 (재부팅 후)

- LXC `onboot=1`
- 내부 systemd `xpra-xdesktop.service` + `nginx.service` 둘 다 enabled
- xpra-xdesktop 이 `/usr/bin/xpra start-desktop --start=xfce4-session` 실행 → XFCE 세션 재시작
- fcitx5 는 유저 autostart 로 (`~/.config/autostart/fcitx5.desktop`)
