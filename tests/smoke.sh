#!/usr/bin/env bash
# pxi-init smoke test — 각 도메인 바이너리 --help + doctor 확인
set -euo pipefail

BIN_DIR="${1:-.}"
FAILED=0
DOMAINS=(code-server wordpress)

for domain in "${DOMAINS[@]}"; do
  bin="${BIN_DIR}/pxi-${domain}"
  echo -n "[${domain}] "

  if [[ ! -x "$bin" ]]; then
    echo "SKIP — ${bin} not found"
    continue
  fi

  # --help 확인
  if "$bin" --help > /dev/null 2>&1; then
    echo -n "help=OK "
  else
    echo "help=FAIL"
    FAILED=1
    continue
  fi

  # doctor 확인 (exit 0)
  if "$bin" doctor > /dev/null 2>&1; then
    echo "doctor=OK"
  else
    echo "doctor=FAIL"
    FAILED=1
  fi
done

if [[ $FAILED -ne 0 ]]; then
  echo -e "\nSMOKE TEST FAILED"
  exit 1
fi

echo -e "\nAll smoke tests passed."
