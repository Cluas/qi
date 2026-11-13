# 「知气 · Qi」部署与上架合规指南（v1.0）

本文档面向项目运维与上架负责人，涵盖三部分：
1. `qi-cloud`（零健康数据库控制面与 ZDR 网关）生产部署；
2. 中国大陆备案与软著申请 SOP；
3. Apple App Store 与 Android Health Connect 审核自查清单。

---

## 一、`qi-cloud` 极简控制面部署

`qi-cloud` 是一个独立 Rust 二进制（源码位于 `/Users/cluas/Codes/qi/cloud`），其 SQLite 中**仅存在三张表**：`accounts`、`monthly_quotas`、`e2ee_sync_envelopes`，**严禁存储任何明文健康指标或体检报告**。所有健康状态都在端侧通过 12 词恢复码派生密钥加密（`XChaCha20-Poly1305`）后再行中继。

### 1.1 核心环境变量（与 `cloud/src/main.rs` 严格一致）

| 变量名 | 默认值 | 说明 |
|---|---|---|
| `QI_CLOUD_BIND` | `127.0.0.1:8788` | 监听地址与端口（容器部署建议 `0.0.0.0:8788`） |
| `QI_CLOUD_DB` | `./qi_control_plane.sqlite3` | 控制面 SQLite 路径 |
| `QI_ZDR_UPSTREAM_URL` | （未设置则本地安全降级） | 上游 OpenAI 兼容 ZDR 模型网关基地址（如阿里云百炼兼容端点、Zero-Retention 推理节点或自建 vLLM/Ollama 节点） |
| `QI_ZDR_UPSTREAM_KEY` | `""` | 上游网关 API Key |
| `QI_ZDR_UPSTREAM_MODEL` | `qwen3:latest` | 默认推理模型 ID |

### 1.2 Docker 部署

```bash
cd /Users/cluas/Codes/qi/cloud
docker build -t qi-cloud:1.0.0 .
docker run -d \
  --name qi-cloud \
  --restart unless-stopped \
  -p 127.0.0.1:8788:8788 \
  -v qi_control_data:/data \
  -e QI_CLOUD_BIND=0.0.0.0:8788 \
  -e QI_CLOUD_DB=/data/qi_control_plane.sqlite3 \
  -e QI_ZDR_UPSTREAM_URL="https://your-zdr-upstream.example.com/v1" \
  -e QI_ZDR_UPSTREAM_KEY="your-upstream-key" \
  -e QI_ZDR_UPSTREAM_MODEL="qwen3-max" \
  qi-cloud:1.0.0
```

### 1.3 Linux systemd + Caddy TLS 反向代理

参考 `cloud/qi-cloud.service` 与 `cloud/Caddyfile.example`：

```bash
cd /Users/cluas/Codes/qi/cloud
cargo build --release
sudo install -m 0755 target/release/qi-cloud /usr/local/bin/qi-cloud
sudo install -d /var/lib/qi-cloud
sudo cp qi-cloud.service /etc/systemd/system/qi-cloud.service
sudo systemctl daemon-reload
sudo systemctl enable --now qi-cloud
```

Caddy 配置 HTTPS（如备案后的大陆节点域名 `api.qi-health.cn`）：
- 自动申请与续签 Let's Encrypt 证书；
- 转发 `/v1/*` 与 `/healthz` 至 `127.0.0.1:8788`；
- 确认上游返回头透传 `Cache-Control: no-store` 与 `X-Qi-Zdr: memory-only-zero-retention`。

### 1.4 数据最小化审计与物理销毁

- 运维可通过 `GET /healthz` 校验 `health_plaintext_retained === false`。
- 用户在 App 点击「物理清空本机健康数据 (5.1.1(v))」时，若已连接控制面会同时调用 `DELETE /v1/account`，服务端立即在事务内删除 `accounts`、`monthly_quotas` 与 `e2ee_sync_envelopes` 下对应账号的所有行。

---

## 二、中国大陆域名与 App 上架备案 SOP

| 步骤 | 办理事项 | 关键材料与注意点 |
|---|---|---|
| 1 | 计算机软件著作权登记 | 软件名称《知气个人健康节律管理软件 V1.0》，提交前/后各 30 页源码（可截取 `src/` 与 `App.tsx`/`pages/`），耗时约 30–60 天（加急 5–15 工作日）。可与服务器搭建同步推进。 |
| 2 | 阿里云/腾讯云域名实名 + ICP 备案 | 购买大陆境内云主机（或挂备案服务号），完成官网与 API 域名（如 `qi-health.cn`）ICP 备案，获取省管局备案号。 |
| 3 | 公安联网备案 | ICP 备案下发 30 日内完成全国互联网安全管理服务平台登记，在官网页脚与隐私政策页悬挂备案编号。 |
| 4 | 工信部 App 备案 | 通过云厂商接入商平台提交 App 名称（知气）、Bundle ID (`eu.org.cluas.qi`) / Android 包名 + SHA256 公钥指纹 + 服务端域名 IP，获取 App 备案号。 |
| 5 | 生成式 AI 备案合规路线 | **强烈建议**：V1.0 的官方 `QI_ZDR_UPSTREAM_URL` 接入**已完成网信办生成式人工智能服务备案的中国大陆大模型 API**（例如百炼 Qwen / 字节豆包 / 智谱 GLM 企业零留存契约版），可走“大模型能力调用方登记”避免独立重训模型备案；纯本地模型与用户自配 Ollama 属端侧自用工具，App 不主动提供公开模型服务。 |

---

## 三、App Store & Android 商店审核自查清单

### 3.1 Apple App Store 审核对齐说明（Submission Notes 模板）

1. **Guideline 5.1.2(i) — AI 第三方数据披露与按需授权**：
   - App 启动、冷启动引导（`OnboardingModal.tsx`）与默认首页计算**零 AI 请求、零数据上传**；
   - 仅当用户主动点击「模拟导入体检单」「AI 实时润色」且当前处于 `official_zdr_cloud` 模式时，才首次弹出 `App.tsx` 中的 `ZDR 零留存授权 (5.1.2i)` 对话框，明确告知发送前先在端侧脱敏 PII，且对方节点为 Zero-Data-Retention，绝不用于模型训练；
   - 切换为「② 纯本机离线大模型」或「③ 家庭局域网 Ollama」后 100% 端侧/局域网执行。
2. **Guideline 5.1.1(v) — 一键账号与数据彻底销毁**：
   - 在「我的 → 物理清空本机健康数据 (5.1.1(v))」提供二次确认后的立即不可逆物理擦除：同时清除本地 SQLite 全部数据并向控制面发送 `DELETE /v1/account` 清理全部远端配额与密文信封。
3. **Guideline 2.5.2 — 严禁运行时下载可执行代码或后台 Metal GPU**：
   - `nsh` 编译输出静态 QuickJS 字节码（`.nshc`），不从网络热更新可执行 JS；
   - `src/ai_provider.rs` 对端侧大模型执行内存（`>= 6GB`）与热降频（`thermal_throttled`）双门禁，且后台预热仅运行纯 Rust SQLite/统计计算，绝不在后台触发 GPU 推理。
4. **HealthKit 与隐私清单（`hosts/ios/PrivacyInfo.xcprivacy`）**：
   - 仅申请读权限（步数、睡眠分析、静息心率、HRV SDNN），不申请写权限或敏感医疗类别；
   - `PrivacyInfo.xcprivacy` 声明仅使用 `NSPrivacyAccessedAPICategoryUserDefaults` 与本地文件时间戳，无广告追踪（IDFA / ATT 不需弹窗）。
5. **非医疗器械免责声明（Medical Disclaimer）**：
   - Rust 常量 `WELLNESS_DISCLAIMER` 在每次简报、隐私白皮书与体检红绿灯页面底栏固定呈现：“仅供参考，不构成临床医疗诊断”。

### 3.2 Android（Google Play & 国内安卓应用市场）自查清单

参考 `hosts/android/AndroidManifest.xml` 与 `MainActivity.kt`：
1. **Health Connect 最小权限**：仅声明 `READ_STEPS`、`READ_SLEEP`、`READ_HEART_RATE`、`READ_HEART_RATE_VARIABILITY`、`READ_RESTING_HEART_RATE`、`READ_EXERCISE`；并在商店填写 Health Connect 隐私说明与使用录屏。
2. **国内市场明文网络**：`nsh.json` 中 `usesCleartextTraffic: true` 仅用于支持家庭局域网用户连入 `http://192.168.x.x:11434` 本地 Ollama，官方控制面强制使用 HTTPS + TLS。
3. **跨端兼容与 12 词恢复**：Android 端提供 WebDAV（坚果云/NAS）、私有 BYO-Server 与 `.qivault` 单文件三种跨生态同步渠道，与 iOS/macOS 共享相同的 HKDF-SHA256 + XChaCha20-Poly1305 信封格式。
