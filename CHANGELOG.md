# Changelog

All notable changes to ODW Recap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> 已知边界（本 release 诚实披露）：本地 whisper.cpp / llama.cpp 转写与摘要引擎为 stub，
> 端到端转写目前依赖云 provider 或远端 Ollama——见
> [team-server/KNOWN_LIMITATIONS.md](team-server/KNOWN_LIMITATIONS.md)。

## [Unreleased] - 2026-09-12

### Changed — UI（评审批次 E）
- **品牌字体真正加载**：index.html 引入 Instrument Serif / Inter / JetBrains Mono，
  Tauri CSP 放行 `fonts.googleapis.com`（style）与 `fonts.gstatic.com`（font）——此前
  `--font-display` 声明了 Instrument Serif 但从未加载，标题实际回退 Times New Roman。
- **暗色模式**：按系统偏好跟随（prefers-color-scheme），暗色令牌对齐 Vault 深色系。
- **状态徽章暖色化**：completed/processing/failed 与 badge 脱离灰蓝 Tailwind 默认色，
  统一套件暖色板（--success/--accent-subtle/--error）。
- **可达性**：底部状态栏 `role="status" aria-live="polite"`（此前读屏完全不可达）、
  全局 `:focus-visible`、`prefers-reduced-motion` 支持。

### Fixed
- DB 连接 SSL 协商失败给出可操作诊断（追加 `?sslmode=disable` 提示，bug 8）。
- 桌面端 `init()` 崩溃（动态渲染按钮的错误绑定顺序，bug 7）。

### Added
- team-server 启动时 `LOOP_WEBHOOK_TRIGGER_ID` 未配置打 WARNING（Recap→Loop 链路
  默认静默关闭的可见化，评审批次 C）。
- README 前置依赖与本地引擎 stub 的诚实声明；本地引擎现状与 KNOWN_LIMITATIONS 互链。

## [1.0.0] - 2026-07-31

### Added
- 桌面端（Tauri/Rust）：会议库、导入/录制、转写/摘要管线、加密存储（AES-256-GCM +
  Argon2id）、模板系统。
- team-server（Go/Chi）：多用户注册/登录（JWT）、会议 CRUD、租户隔离、审计日志、
  `POST /sync` → Vault `/files/upload` 真实入库、Loop webhook HMAC-SHA256 触发。
- ODW.ai 品牌对齐（logo/色板/odw.ai 链接）。
