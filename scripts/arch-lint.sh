#!/usr/bin/env bash
# pxi 아키텍처 린트 — 도메인 구조 규약 전수 검증.
#
# 검사 항목:
#   1. crates/domains/*/ 마다 domain.ncl 존재 여부
#   2. ncl/domains.ncl import 목록 ↔ 실제 디렉토리 완전 일치 (누락·고아)
#   3. Cargo.toml: name = "pxi-<dirname>", version/edition.workspace = true
#   4. src/main.rs 첫 3줄에 //! 모듈 doc 존재
#   5. nickel eval ncl/domains.ncl (Domain contract 전수 검증, nickel 있으면)
#
# 사용법:
#   scripts/arch-lint.sh           — 전체 검사
#   scripts/arch-lint.sh --fast    — nickel eval 생략 (빌드 없이 구조만 검사)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

FAIL=0
ok()   { printf '  \033[32m✓\033[0m %s\n' "$1"; }
ng()   { printf '  \033[31m✗\033[0m %s\n' "$1"; FAIL=1; }
warn() { printf '  \033[33m⚠\033[0m %s\n' "$1"; }

# ─── 1. domain.ncl 존재 여부 ─────────────────────────────────────────────────
echo "=== 1. domain.ncl 존재 여부 ==="
MISSING_CARD=0
for d in crates/domains/*/; do
    name=$(basename "$d")
    if [[ ! -f "$d/domain.ncl" ]]; then
        ng "$name: domain.ncl 없음 — scripts/new-domain.sh 로 생성하거나 직접 작성"
        MISSING_CARD=$((MISSING_CARD + 1))
    fi
done
[[ "$MISSING_CARD" == "0" ]] && ok "모든 도메인 domain.ncl 존재"

# ─── 2. ncl/domains.ncl import ↔ 디렉토리 동기화 ────────────────────────────
echo ""
echo "=== 2. ncl/domains.ncl ↔ crates/domains/ 동기화 ==="
SYNC_FAIL=0

# 실제 디렉토리 이름 목록
actual_dirs=()
for d in crates/domains/*/; do
    actual_dirs+=("$(basename "$d")")
done

# domains.ncl 에서 import 되는 이름 추출 (key = import 패턴)
imported_names=()
while IFS= read -r line; do
    # "  lxc = import" 또는 "  chrome-browser-dev = import" 패턴
    if [[ "$line" =~ ^[[:space:]]+([a-z][a-z0-9-]+)[[:space:]]*=[[:space:]]*import ]]; then
        imported_names+=("${BASH_REMATCH[1]}")
    fi
done < ncl/domains.ncl

# 디렉토리에 있지만 import 안 된 것 (누락)
for name in "${actual_dirs[@]}"; do
    found=0
    for imp in "${imported_names[@]}"; do
        [[ "$imp" == "$name" ]] && found=1 && break
    done
    if [[ "$found" == "0" ]]; then
        ng "ncl/domains.ncl 에 '$name' import 누락 — domains.ncl 에 추가 필요"
        SYNC_FAIL=$((SYNC_FAIL + 1))
    fi
done

# import 됐지만 디렉토리 없는 것 (고아)
for imp in "${imported_names[@]}"; do
    found=0
    for name in "${actual_dirs[@]}"; do
        [[ "$name" == "$imp" ]] && found=1 && break
    done
    if [[ "$found" == "0" ]]; then
        ng "ncl/domains.ncl 에 '$imp' import 있으나 crates/domains/$imp/ 없음 — 고아 import 제거 필요"
        SYNC_FAIL=$((SYNC_FAIL + 1))
    fi
done

[[ "$SYNC_FAIL" == "0" ]] && ok "ncl/domains.ncl ↔ crates/domains/ 완전 일치"

# ─── 3. Cargo.toml 규약 ─────────────────────────────────────────────────────
echo ""
echo "=== 3. Cargo.toml 규약 (name / workspace 상속) ==="
CARGO_FAIL=0
for d in crates/domains/*/; do
    name=$(basename "$d")
    cargo="$d/Cargo.toml"
    [[ ! -f "$cargo" ]] && ng "$name: Cargo.toml 없음" && CARGO_FAIL=$((CARGO_FAIL + 1)) && continue

    cargo_name=$(grep -m1 '^name' "$cargo" | awk -F'"' '{print $2}' 2>/dev/null || echo "")
    if [[ "$cargo_name" != "pxi-$name" ]]; then
        ng "$name: Cargo.toml name = \"$cargo_name\" (기대: \"pxi-$name\")"
        CARGO_FAIL=$((CARGO_FAIL + 1))
    fi
    if ! grep -q 'version\.workspace\s*=\s*true' "$cargo"; then
        ng "$name: version.workspace = true 없음 — 버전 워크스페이스 상속 필수"
        CARGO_FAIL=$((CARGO_FAIL + 1))
    fi
    if ! grep -q 'edition\.workspace\s*=\s*true' "$cargo"; then
        ng "$name: edition.workspace = true 없음 — 에디션 워크스페이스 상속 필수"
        CARGO_FAIL=$((CARGO_FAIL + 1))
    fi
    if ! grep -q 'pxi-core' "$cargo"; then
        ng "$name: pxi-core 의존 없음 — 모든 도메인은 pxi-core에 의존해야 함"
        CARGO_FAIL=$((CARGO_FAIL + 1))
    fi
done
[[ "$CARGO_FAIL" == "0" ]] && ok "전체 Cargo.toml 규약 통과"

# ─── 4. src/main.rs //! 모듈 doc ─────────────────────────────────────────────
echo ""
echo "=== 4. src/main.rs //! 모듈 doc 주석 ==="
DOC_FAIL=0
for d in crates/domains/*/; do
    name=$(basename "$d")
    main="$d/src/main.rs"
    [[ ! -f "$main" ]] && ng "$name: src/main.rs 없음" && DOC_FAIL=$((DOC_FAIL + 1)) && continue
    if ! head -3 "$main" | grep -q '^//!'; then
        ng "$name: src/main.rs 첫 3줄에 //! 모듈 doc 없음 — '//! pxi-$name — ...' 형식 필수"
        DOC_FAIL=$((DOC_FAIL + 1))
    fi
done
[[ "$DOC_FAIL" == "0" ]] && ok "전체 도메인 //! doc 존재"

# ─── 5. nickel eval — Domain contract 전수 검증 ──────────────────────────────
echo ""
echo "=== 5. nickel eval ncl/domains.ncl ==="
if [[ "$FAST" == "1" ]]; then
    warn "--fast 모드 — nickel eval 생략"
elif ! command -v nickel &>/dev/null; then
    warn "nickel CLI 없음 — skip (빌드 환경에서는 build.rs 가 대신 검증)"
else
    if nickel eval --format json ncl/domains.ncl > /dev/null 2>&1; then
        ok "nickel Domain contract 전수 통과"
    else
        ng "nickel eval 실패 — domain.ncl 규약 위반"
        nickel eval --format json ncl/domains.ncl 2>&1 | head -20
        FAIL=1
    fi
fi

# ─── 결과 ─────────────────────────────────────────────────────────────────────
echo ""
if [[ "$FAIL" != "0" ]]; then
    echo -e "\033[31m=== ✗ arch-lint 실패 ===\033[0m"
    echo "  도메인 추가 시: scripts/new-domain.sh <name> <product> <layer> <platform> \"<설명>\""
    exit 1
fi
echo -e "\033[32m=== ✓ arch-lint 전체 통과 ===\033[0m"
