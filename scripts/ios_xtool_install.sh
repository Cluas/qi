#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW_FILE="ios-package.yml"
ARTIFACT_NAME="qi-ios-unsigned"
DOWNLOAD_DIR="${REPO_ROOT}/.nsh/out/ios-ci"
BUNDLE_ID="eu.org.cluas.qi"

DO_BUILD=false
DO_LAUNCH=false
DRY_RUN=false
IPA_PATH=""
UDID=""
TARGET_REF=""
RESOLVED_REF=""

usage() {
  cat <<'EOF'
《知气（Qi）》云端 iOS 取包 + 本机 xtool 签名装真机脚本

用法:
  ./scripts/ios_xtool_install.sh [选项]

选项:
  --build          触发 GitHub Actions 手动打包并等待完成后下载产物（Public 仓享无限免费 macOS 构建额度）
  --ref <branch>   指定触发云端打包的远端分支（默认取当前分支；若处于空壳 main 则自动回退到 dev）
  --ipa <path>     跳过 GitHub 下载，直接安装指定的本地 .ipa 或 .app 文件
  --udid <udid>    多台设备连接时指定目标 iPhone UDID（透传 xtool -u）
  --launch         安装完成后立即在真机启动《知气》（eu.org.cluas.qi）
  --dry-run        仅检查本机 gh / git remote / xtool 就绪状态，不触发云端或装机
  -h, --help       显示本帮助信息

示例:
  ./scripts/ios_xtool_install.sh --dry-run
  ./scripts/ios_xtool_install.sh --build --launch
  ./scripts/ios_xtool_install.sh --build --ref dev --launch
  ./scripts/ios_xtool_install.sh --ipa ~/Downloads/qi-ios-unsigned.ipa --launch
EOF
}

resolve_xtool() {
  if command -v xtool >/dev/null 2>&1; then
    command -v xtool
    return 0
  fi
  local app_bin="/Applications/xtool.app/Contents/Resources/bin/xtool"
  if [ -x "${app_bin}" ]; then
    echo "${app_bin}"
    return 0
  fi
  return 1
}

check_github_ready() {
  if ! command -v gh >/dev/null 2>&1; then
    echo "❌ 未检测到 GitHub CLI (gh)。请先执行: brew install gh" >&2
    return 1
  fi
  if ! gh auth status >/dev/null 2>&1; then
    echo "❌ GitHub CLI 尚未登录。请先执行: gh auth login" >&2
    return 1
  fi
  local origin_url
  origin_url="$(git -C "${REPO_ROOT}" remote get-url origin 2>/dev/null || true)"
  if [ -z "${origin_url}" ]; then
    echo "❌ 当前仓库尚未配置 origin 远端，请先创建 Public GitHub 仓库并按序推送 main 与 dev 双分支：" >&2
    echo "   gh repo create qi --public --source=. --remote=origin -d \"Qi (知气)\"" >&2
    echo "   git push -u origin main && git push -u origin dev" >&2
    return 1
  fi
  echo "✓ GitHub 远端就绪: ${origin_url}"
}

check_xtool_ready() {
  local xtool_bin
  if ! xtool_bin="$(resolve_xtool)"; then
    echo "❌ 未检测到 xtool。请按以下步骤一次性安装（约 21 MB，无需 Xcode）：" >&2
    echo "   1) brew install --cask xtool-org/tap/xtool" >&2
    echo "   2) xtool setup   （登录模式选择 0: API Key，填入付费开发者账号的 Key ID / Issuer ID / .p8 路径）" >&2
    return 1
  fi
  echo "✓ 检测到 xtool: ${xtool_bin}"

  local auth_out
  auth_out="$("${xtool_bin}" auth status 2>&1 || true)"
  if echo "${auth_out}" | grep -qi "Logged in"; then
    echo "✓ xtool Apple Developer 认证已登录"
  else
    echo "❌ xtool 尚未完成 Apple Developer 认证。请执行:" >&2
    echo "   xtool setup   （选择 0: API Key，填入 App Manager 团队密钥）" >&2
    return 1
  fi
}

run_gh() {
  (cd "${REPO_ROOT}" && gh "$@")
}

find_latest_run_id() {
  local conclusion_filter="${1:-}"
  if [ -n "${conclusion_filter}" ]; then
    run_gh run list --workflow "${WORKFLOW_FILE}" --limit 20 --json databaseId,conclusion \
      --jq "[.[] | select(.conclusion == \"${conclusion_filter}\")][0].databaseId // empty"
  else
    run_gh run list --workflow "${WORKFLOW_FILE}" --limit 1 --json databaseId \
      --jq '.[0].databaseId // empty'
  fi
}

get_current_branch() {
  local branch
  branch="$(git -C "${REPO_ROOT}" branch --show-current 2>/dev/null || true)"
  if [ -z "${branch}" ]; then
    branch="$(git -C "${REPO_ROOT}" rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
  fi
  echo "${branch}"
}

resolve_target_ref() {
  if [ -n "${TARGET_REF}" ]; then
    RESOLVED_REF="${TARGET_REF}"
    return 0
  fi

  local current_branch
  current_branch="$(get_current_branch)"
  if [ ! -f "${REPO_ROOT}/nsh.json" ]; then
    if [ "${current_branch}" = "main" ]; then
      echo "ℹ️  当前位于空壳占位 'main' 分支（缺少 nsh.json），已自动回退云端打包目标分支为 'dev'。"
      RESOLVED_REF="dev"
      return 0
    fi
    echo "❌ 当前目录缺少 nsh.json（当前分支: ${current_branch:-unknown}），无法作为完整应用源码打包。" >&2
    echo "   请先执行 git checkout dev，或显式传入 --ref dev。" >&2
    exit 1
  fi

  if [ -z "${current_branch}" ] || [ "${current_branch}" = "HEAD" ]; then
    echo "❌ 无法识别当前本地分支（detached HEAD），请先切换到 dev 分支或传入 --ref dev。" >&2
    exit 1
  fi

  RESOLVED_REF="${current_branch}"
}

trigger_and_watch_build() {
  local ref prev_run_id new_run_id attempt
  resolve_target_ref
  ref="${RESOLVED_REF}"

  if [ -n "$(git -C "${REPO_ROOT}" status --porcelain)" ]; then
    echo "⚠️  检测到本地存在未提交改动；GitHub Actions 仅会打包已 push 到远端 '${ref}' 的提交。"
  fi

  prev_run_id="$(find_latest_run_id)"
  echo "→ 正在手动触发远端工作流 ${WORKFLOW_FILE} (ref: ${ref})..."
  run_gh workflow run "${WORKFLOW_FILE}" --ref "${ref}"

  new_run_id=""
  for attempt in $(seq 1 20); do
    sleep 2
    local candidate
    candidate="$(find_latest_run_id)"
    if [ -n "${candidate}" ] && [ "${candidate}" != "${prev_run_id}" ]; then
      new_run_id="${candidate}"
      break
    fi
  done

  if [ -z "${new_run_id}" ]; then
    echo "❌ 触发工作流后未能获取新的 Run ID，请运行 gh run list --workflow ${WORKFLOW_FILE} 查看。" >&2
    exit 1
  fi

  echo "→ 正在等待云端构建 Run #${new_run_id} 完成..."
  run_gh run watch "${new_run_id}" --exit-status
  echo "${new_run_id}" > "${DOWNLOAD_DIR}/.last_run_id"
}

download_ipa_from_run() {
  local run_id="$1"
  rm -rf "${DOWNLOAD_DIR}"
  mkdir -p "${DOWNLOAD_DIR}"
  echo "→ 正在从 Run #${run_id} 下载产物 ${ARTIFACT_NAME}..."
  run_gh run download "${run_id}" -n "${ARTIFACT_NAME}" -D "${DOWNLOAD_DIR}"

  local found_ipa
  found_ipa="$(find "${DOWNLOAD_DIR}" -maxdepth 2 -name '*.ipa' | head -n 1)"
  if [ -z "${found_ipa}" ] || [ ! -f "${found_ipa}" ]; then
    echo "❌ 已下载 Run #${run_id} 产物，但在 ${DOWNLOAD_DIR} 下未找到 .ipa 文件。" >&2
    exit 1
  fi
  IPA_PATH="${found_ipa}"
}

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      -h|--help)
        usage
        exit 0
        ;;
      --build)
        DO_BUILD=true
        shift
        ;;
      --ref)
        if [ "$#" -lt 2 ] || [ -z "${2:-}" ]; then
          echo "❌ --ref 需要传入远端分支名。" >&2
          exit 1
        fi
        TARGET_REF="$2"
        shift 2
        ;;
      --ipa)
        if [ "$#" -lt 2 ] || [ -z "${2:-}" ]; then
          echo "❌ --ipa 需要传入 .ipa 或 .app 路径。" >&2
          exit 1
        fi
        IPA_PATH="$2"
        shift 2
        ;;
      --udid)
        if [ "$#" -lt 2 ] || [ -z "${2:-}" ]; then
          echo "❌ --udid 需要传入设备 UDID。" >&2
          exit 1
        fi
        UDID="$2"
        shift 2
        ;;
      --launch)
        DO_LAUNCH=true
        shift
        ;;
      --dry-run)
        DRY_RUN=true
        shift
        ;;
      *)
        echo "❌ 未知参数: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
  done
}

main() {
  parse_args "$@"

  if [ "${DRY_RUN}" = true ]; then
    echo "=== 《知气》iOS xtool 装机环境预检 (--dry-run) ==="
    local gh_ok=0 xtool_ok=0
    local local_branch
    local_branch="$(get_current_branch)"
    resolve_target_ref

    if [ -z "${IPA_PATH}" ]; then
      check_github_ready || gh_ok=1
    else
      echo "✓ 已指定本地安装包: ${IPA_PATH}"
    fi
    check_xtool_ready || xtool_ok=1

    if command -v nsh >/dev/null 2>&1; then
      echo "✓ 本机 NSH CLI: $(nsh --version)"
    fi
    echo "→ 当前本地分支: ${local_branch:-unknown}"
    echo "→ 云端打包目标 ref: ${RESOLVED_REF}"
    echo "→ 计划模式: $([ "${DO_BUILD}" = true ] && echo "触发云端打包 (--build, ref: ${RESOLVED_REF}) 后装机" || ([ -n "${IPA_PATH}" ] && echo '安装本地指定包' || echo '复用最近一次成功云端构建产物（0 分钟额度消耗）'))"
    [ "${DO_LAUNCH}" = true ] && echo "→ 安装后自动拉起: ${BUNDLE_ID}"
    [ -n "${UDID}" ] && echo "→ 指定设备 UDID: ${UDID}"

    if [ "${gh_ok}" -eq 0 ] && [ "${xtool_ok}" -eq 0 ]; then
      echo "✓ 所有就绪项检查通过，可随时执行实际装机。"
    else
      echo "ℹ️  请按上方提示补齐未就绪项后再执行正式安装。"
    fi
    exit 0
  fi

  local xtool_bin
  if ! check_xtool_ready; then
    exit 1
  fi
  xtool_bin="$(resolve_xtool)"

  if [ -z "${IPA_PATH}" ]; then
    check_github_ready
    mkdir -p "${DOWNLOAD_DIR}"
    if [ "${DO_BUILD}" = true ]; then
      trigger_and_watch_build
      local built_id
      built_id="$(cat "${DOWNLOAD_DIR}/.last_run_id")"
      download_ipa_from_run "${built_id}"
    else
      local latest_ok_id
      latest_ok_id="$(find_latest_run_id "success")"
      if [ -z "${latest_ok_id}" ]; then
        echo "❌ 尚未找到成功的 ${WORKFLOW_FILE} 构建记录。" >&2
        echo "   请先执行: ./scripts/ios_xtool_install.sh --build" >&2
        exit 1
      fi
      download_ipa_from_run "${latest_ok_id}"
    fi
  elif [ ! -e "${IPA_PATH}" ]; then
    echo "❌ 指定的安装包不存在: ${IPA_PATH}" >&2
    exit 1
  fi

  echo "→ 扫描已连接 iOS 设备（请确保 iPhone 已 USB 连接、解锁并信任此电脑）..."
  "${xtool_bin}" devices --no-wait || true

  local -a conn_args=()
  if [ -n "${UDID}" ]; then
    conn_args=(-u "${UDID}")
  fi

  echo "→ 正在使用付费开发者账号签名并安装 ${IPA_PATH}..."
  if [ "${#conn_args[@]}" -gt 0 ]; then
    "${xtool_bin}" install "${conn_args[@]}" "${IPA_PATH}"
  else
    "${xtool_bin}" install "${IPA_PATH}"
  fi

  if [ "${DO_LAUNCH}" = true ]; then
    echo "→ 正在真机启动 ${BUNDLE_ID}..."
    if [ "${#conn_args[@]}" -gt 0 ]; then
      "${xtool_bin}" launch "${conn_args[@]}" "${BUNDLE_ID}"
    else
      "${xtool_bin}" launch "${BUNDLE_ID}"
    fi
  fi

  echo "✅ 完成！《知气》已签名安装到你的 iPhone。"
}

main "$@"
