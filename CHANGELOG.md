# Changelog

All notable changes to ODW Recap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> 已知边界（本 release 诚实披露）：本地 whisper.cpp / llama.cpp 转写与摘要引擎为 stub，
> 端到端转写目前依赖云 provider 或远端 Ollama——见
> [team-server/KNOWN_LIMITATIONS.md](team-server/KNOWN_LIMITATIONS.md)。

## [Unreleased] - 2026-09-13

### Security / Changed — 隐私与前端
- **桌面端不再于启动时请求 Google Fonts**：字体改为**本地自托管**（`src/fonts.css` +
  `src/assets/fonts/*.woff2`，Inter / JetBrains Mono 为 variable 单文件，Latin +
  Latin-Ext 子集，约 200KB）。此前 `批次 E` 通过远程 CDN 加载，与"on-device /
  sovereign"定位相悖（每次启动向 Google 泄漏 IP/UA/使用信号，且离线时字体回退）。
  视觉结果保持一致。
- **CSP 收紧**：`style-src 'self'`（去掉 `unsafe-inline` 与远程 host）、
  `font-src 'self'`。新增 Jest 守卫：`<link|script|img>` 不得引用远程资源，
  `fonts.css` 只允许本地 `assets/fonts/`（回归即失败）。
- **拖拽导入多文件**：`tauri://file-drop` 现导入全部文件而非仅第一个。

### Added — team-server 可审计性
- **审计日志填充 `details` JSONB**：meeting.create/update 记录标题、meeting.sync
  记录 action（此前该列恒为空）。
- **sync_queue 外键回退**：客户端传入非 UUID / 不存在的 meeting id 时，
  以 NULL `meeting_id` 落库，保证同步审计不丢（此前静默丢弃）。

### Fixed
- `team-server/cmd/server/main.go` 经 `gofmt` 规范化。

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
