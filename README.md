# 知气 · Qi

跨平台大众健康管家客户端（基于 [`longbridge/nsh`](https://github.com/longbridge/nsh) `Rust + TSX` 混合架构）。

- **Data Local-First**：生理序列、饮食日志与体检档案默认存储在本机 SQLite 中，可加密切分后同步至个人云（iCloud Drive / WebDAV / 自托管 `BYO-Server`），官方控制面零留存个人健康库。
- **AI Local-Optional**：端侧 Rust 确定性统计底座负责 Z-Score、ACWR 与 `RMSSD/SDNN` 严格隔离计算；支持官方 ZDR 零留存云、纯本机离线大模型（iOS `FoundationModels` / Phase 2 内嵌 `Qwen3`）与家庭局域网 `Ollama` 无缝切换。

## 工程结构

- `nsh.json` / `theme.json` / `DESIGN.md`：宋韵现代节律语义主题 Token（浅色宣纸 / 深色玄石）与动效规范
- `src/analytics.rs`：30 天个人基线、7d/28d ACWR、睡眠蓄水池、三档硬件自适应降级与 `D -> D+1` 滞后归因
- `src/crypto_sync.rs`：`XChaCha20-Poly1305` 设备隔离加密增量分片与 12 词恢复码主密钥派生
- `src/safety_ai.rs`：端侧化验单前置 PII 遮盖、Rust 参考区间纠偏、SaMD 医疗红线过滤与 AI 引擎路由
- `src/storage.rs`：`rusqlite` 本地持久化存储与 35 天节律演示数据集
- `src/lib.rs`：`#[native_module("qi/native")]` 导出 `QiEngine` 与 `QiMeta`
- `ui/components.tsx` / `store.ts` / `pages/`：60 秒冷启动引导与 3-Tab（今日元气 / 记录与体检 / 我的）

## 本地开发与验证

```sh
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

# 1. Rust 确定性算法、加密同步与合规护栏单元测试
cargo test

# 2. Rust + TSX 强类型检查、打包诊断与无头端到端 UI 测试
nsh check
nsh build
nsh test

# 3. 本地桌面原生运行（macOS AppKit）
nsh run
```

> **开源致谢**：本项目遵循 Apache-2.0 协议并保留 [`NOTICE`](NOTICE) 中对 `PulsHealth` 与 `Native Shell (NSH)` 贡献者的归属声明；全端文案严守非医疗器械参考边界。
