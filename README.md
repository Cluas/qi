# 知气 · Qi（v1.0）

跨平台大众健康与个人节律管家客户端及零健康库控制面（基于 [`longbridge/nsh`](https://github.com/longbridge/nsh) `Rust + TSX` 混合原生架构）。

- **Data Local-First**：生理序列、饮食日志、N-of-1 微习惯实验与体检档案默认保存在本机 `rusqlite` SQLite；使用本机 12 词恢复码派生 `HKDF-SHA256 + XChaCha20-Poly1305` 密钥，支持 `iCloud Drive`（默认免费）、`WebDAV`（坚果云 / NAS）、私有/官方中继 `BYO-Server` 以及 `.qivault` 单文件备份与一键换机恢复。
- **AI Local-Optional**：端侧 Rust 确定性统计引擎完成所有基线、ACWR 与临床区间计算；AI 解读层可在「官方 ZDR 零留存网关（Apple 5.1.2(i) 懒授权）」「纯本机离线算力（Apple Vision OCR + FoundationModels / RAM 与热降频门禁）」「家庭局域网 Ollama（OpenAI 兼容 `/v1/chat/completions`）」之间一键切换。

---

## 全栈工程结构

```text
qi/
├── src/                        # 端侧 Rust 核心引擎 (nsh-app-qi, #[native_module("qi/native")])
│   ├── analytics.rs            # 30 天个人基线、7d/28d ACWR、睡眠蓄水池、Tier A/B/C 降级、D→D+1 滞后归因
│   ├── nutrition_db.rs         # 60+ 中式高频餐饮/饮品/补剂词典 + 0ms 离线自然语言解析
│   ├── safety_ai.rs            # 端侧 PII 前置脱敏、25+ 体检指标解析/区间校验/跨期趋势、<think> 与 SaMD 红线过滤
│   ├── experiments.rs          # N-of-1 微习惯对照实验与 7 天周期对比引擎
│   ├── apple_native.rs         # Apple Vision 离线 OCR 与 FoundationModels / Ollama 算力探针桥接
│   ├── ai_provider.rs          # 4 档 AI 网关客户端 (lan_ollama / official_zdr_cloud / on_device_local / deterministic)
│   ├── importers.rs            # Apple Health export.xml 片段、CSV 与 JSONL 导入（严格隔离 RMSSD 与 SDNN）
│   ├── crypto_sync.rs          # HKDF-SHA256 + XChaCha20-Poly1305 设备隔离分片加解密
│   ├── sync_engine.rs          # iCloud Drive / WebDAV / BYO-Server 真实推拉、去重合并与 .qivault 归档
│   └── storage.rs              # SQLite 持久化、35 天节律演示集与 App 物理销毁
├── cloud/                      # 零健康明文数据库控制面 + ZDR 无状态代理 (qi-cloud, Axum 0.8 + SQLite)
├── ui/
│   ├── components.tsx          # 宋韵现代语义组件库（Card / VitalBadge / SheetDialog 等）
│   └── charts.tsx              # 原生 Canvas 矢量图表（270° 元气仪表面环、ACWR 甜区轨道、14 天平滑曲线）
├── pages/
│   ├── OnboardingModal.tsx     # 60 秒零术语无感冷启动引导
│   ├── TodayPage.tsx           # Tab 1：今日元气、Tier 降级、唯一高杠杆行动、AI 实时润色与 14 天趋势
│   ├── LogLabsPage.tsx         # Tab 2：3 秒记餐、60+ 中式词典、Apple Vision 离线 OCR、25+ 体检红绿灯与导入器
│   ├── InsightsPage.tsx        # Tab 3：近 7 天 vs 前 7 天对照、N-of-1 微习惯实验室与滞后归因
│   └── MeVaultPage.tsx         # Tab 4：控制面会话、E2EE 同步与 12 词恢复、算力探针、早安通知与 5.1.1(v) 清空
├── hosts/                      # 原生宿主构建套件（macOS Vision Helper / iOS HealthKit & Vision / Android Health Connect）
├── scripts/generate_assets.swift # CoreGraphics 1024×1024 宋韵品牌图标与测试体检单图片生成器
└── docs/DEPLOYMENT_AND_FILING.md # qi-cloud 部署、中国大陆备案与 App Store / Android 合规 SOP
```

---

## 本地开发、测试与打包

```sh
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

# 1. 编译 macOS 原生 Apple Vision / FoundationModels 辅助二进制（可选增强）
./hosts/macos/build_helper.sh

# 2. Rust 核心引擎与 qi-cloud 控制面测试
cargo test
(cd cloud && cargo test)

# 3. Rust + TSX 类型检查、Bytecode 构建与无头端到端 UI 测试
nsh check
nsh build
QI_DB_PATH=:memory: nsh test

# 4. 打包独立可安装 macOS 应用 (.nsh/out/知气 · Qi.app)
nsh package --target macos

# 5. 本地启动控制面并桌面运行客户端
(cd cloud && cargo run &)
nsh run
```

> **开源致谢与免责**：本项目遵循 Apache-2.0 协议并保留 [`NOTICE`](NOTICE) 中对 `PulsHealth` 与 `Native Shell (NSH)` 贡献者的归属声明；全端健康评分与趋势分析仅供个人健康节律参考，不构成医疗诊断。
