#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${HOME}/.qi/bin"
LOCAL_BIN="${ROOT_DIR}/bin"
mkdir -p "${OUT_DIR}" "${LOCAL_BIN}"

swiftc -O \
  "${ROOT_DIR}/hosts/macos/qi_vision_helper.swift" \
  -o "${OUT_DIR}/qi-vision-helper"

cp "${OUT_DIR}/qi-vision-helper" "${LOCAL_BIN}/qi-vision-helper"
chmod +x "${OUT_DIR}/qi-vision-helper" "${LOCAL_BIN}/qi-vision-helper"
echo "Built ${OUT_DIR}/qi-vision-helper and ${LOCAL_BIN}/qi-vision-helper"
