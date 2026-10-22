# Changelog

All notable changes to ODW Recap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> 已知边界（本 release 诚实披露）：本地 whisper.cpp / llama.cpp 转写与摘要引擎为 stub，
> 端到端转写目前依赖云 provider 或远端 Ollama——见
> [team-server/KNOWN_LIMITATIONS.md](team-server/KNOWN_LIMITATIONS.md)。

## [Unreleased] - 2026-10-22（第六轮深度审查）

### Added
- **手机上传页 `GET /upload/audio`（R-2）**：极简 HTML 上传表单（token 隐藏域内联），
  USER-GUIDE §6 "手机浏览器打开网址上传" 全流程首次真正可用。
- **`search_transcripts` 命令 + 会议库搜索框（R-6）**：FTS5 全文检索端到端可用——
  save_transcript_segments 事务内维护外部内容索引（旧行 delete 命令退役 + 新行
  索引），查询修正 join（tseg.id 而非 rowid 错位）+ rank 排序 + 高亮片段；
  会议库头部回车搜索并点击跳转详情。

### Fixed
- **[P0] 手机上传链路断裂（R-2）**：上传只写文件并返回伪造 meeting_id（不存在于任何
  表）——文件永不被处理、会议库永不可见。现复用入库助手创建真实会议行（标题/
  音频路径/大小/来源=MobileUpload），并校验扩展名（白名单 m4a/wav/mp3/ogg/webm/flac）。
- **[P0] 云端 LLM 摘要行动项恒空（R-3）**：openai/anthropic/ollama 返回自由文本而
  action_items/decisions 硬编码空——UI 显示 "No action items detected"，且
  action_items 为空使 Recap→Loop webhook 链路永不触发。现 prompt 请求严格 JSON
  （STRUCTURED_OUTPUT_INSTRUCTION 公共层），解析容忍 markdown 围栏/散文包裹/
  非字符串数组项，失败回退规则提取器（行动项永不静默消失）。
- **[P1] 监听文件夹 provider 配置（R-4）**：导入硬编码 whisper_local（stub）必然
  "not available yet"；现从 Settings 读取 stt/llm provider。
- **[P1] 桌面同步先建会（R-5）**：sync 从不 POST /meetings——服务端 FK 违例回退
  NULL 会议入队，会议永不出现在团队库/管理统计。现先 upsert（409=已存在放行）再
  /sync。
- **[P2] worker 重复消费（R-7）**：claim 先 commit 再转发，锁即刻释放——双副本下
  同一行被重复转发（重复 Vault 文档）。claim 事务内置 processing + 10 分钟租约，
  过期可重领。
- **[P2] truncate 多字节 panic（R-8）**：错误体含中文且 300 字节边界切在字符中间时
  panic；char_boundary 安全截断。
- **[P2] JWT 过期恢复（R-9）**：24h 后所有同步 401 且 "log out first" 阻塞重登；
  401 时清 token + login 直接覆盖旧 token。
- **[P2] 首个注册用户自动 admin（R-10）**：注册硬编码 member 且无任何提权端点，
  /admin/* 与 RBAC 需手工 SQL 才可用。
- **[P2] /sync 属主校验（R-18）**：Get 返回任意 DB 错误时静默跳过租户校验放行；
  现仅 ErrMeetingNotFound 放行（未知 id 建审计行），其余 500。
- **[P2] Deepgram（R-12）**：请求现携带 per-meeting language（FR-TR-15）；分段置信度
  均值用各词自身值（此前用最后一个词的值）。
- **[P2] 设置页诚实标注（R-11）**：无实现的 STT/LLM 选项标注 "not yet available"
  并禁用；本地转写错误信息携带用户实际选择的 provider 名。新增 rule_based 选项。
- **[P2] 监听文件夹持久化（R-15）**：start/stop 现写 watch_folder_enabled/path
  （此前字段零使用，重启丢失）。
- **[P2] ratelimit 注释与实现对齐（R-17）**。

### Security / 诚实披露
- **[P0→披露] R-1 数据落盘加密范围**：仅凭据（API key）为 AES-256-GCM；会议音频/
  转写/摘要当前明文存于 ~/RecapData。全库加密（SQLCipher + inbox 加密 + 管线透明
  解密）列 V1.7——半启用的加密（管线读不回）比当前诚实状态更糟。README 与
  KNOWN_LIMITATIONS.md 已精确标注范围与 OS 层缓解（FileVault/BitLocker）。

### Deferred（V1.7）
- R-13 Whisper 25MB 分段、R-14 提示词模板接入管线、R-16 桌面离线队列、R-19 watch
  轮询移出回调线程——登记于根仓库 REVIEW_ROUND6_DISCUSSION.md §三。

## [Unreleased] - 2026-10-08（上线前深度审查轮）

### Fixed — team-server (Go)
- **[P1] `POST /sync` 无属主校验（租户隔离缺口）**：`/meetings/*` 走 `canAccess` 而 `/sync`
  从不查会议属主——任何已认证用户可对他人会议 UUID 提交同步（伪造审计归因、向
  `sync_queue` 写入引用他人会议的行、以受害者会议 ID 落盘 Vault 文档）。现对已存在的
  会议 UUID 先取回并 `canAccess` 校验（403），新 UUID 保持放行（create-then-sync 流）。
- **[P1] `sync_queue` 只写不消费**：新增队列 worker（`internal/sync/worker.go`）——
  `FOR UPDATE SKIP LOCKED` 领取到期行（多副本安全），指数退避重试（30s 起、5 次封顶、
  `next_attempt_at` 列），成功置 `processed`；handler 内联转发后同步落终态，崩溃遗留的
  超时 `pending` 行也会被接管。瞬时 Vault/Loop 故障不再静默丢数据。
- **[P1] 超时层级冲突**：`WriteTimeout 15s` < 转发最坏 60s——客户端看到失败而服务端实际
  完成并因重试在 Vault 产生重复文档。WriteTimeout 提至 70s（> chi 60s handler 预算）。
- **[P2] 登录账号枚举时间侧信道**：邮箱不存在时先对固定 dummy bcrypt 哈希做一次比较，
  两分支耗时一致。

### Fixed — 桌面端 (Rust/JS)
- **[P1] 桌面端→team-server 同步是"假成功"空操作**：`SyncClient` 全部为注释占位，
  `SyncEngine` 全仓零引用。现实现真实客户端（`POST /auth/login` 取 JWT → Bearer
  `POST /sync` 全量负载：会议元数据 + 转写分段 + 摘要 + action_items（Loop 触发键）），
  新增 Tauri 命令 `team_server_login/logout/status` 与 `sync_meeting_to_team_server`
  （同步后回写本地 `sync_status`）；设置页新增连接区，详情页新增 Sync 按钮。
- **[P1] Tauri allowlist 未开 dialog**：文件/文件夹选择器被 IPC 拒绝（`dialog.open not
  allowed`）——UI 导入路径全失效。allowlist + Cargo feature 补 `dialog-open`。
- **[P1] 拖拽导入调用不存在的 `importAudioPaths`**：`tauri://file-drop` 抛
  ReferenceError 后文件被静默丢弃。补齐该函数（逐个走 `importAudioPath`）。
- **[P1] LAN 上传受 axum 2MB 默认 body 限制**：真实录音（1h m4a ≈ 30MB）必超限，且
  multipart 错误被吞成误导性 "No file provided"。`DefaultBodyLimit` 提至 500MB，错误
  带真实原因向上传播。
- **[P2] STT/LLM provider HTTP 客户端无超时**：13 处 `reqwest::Client::new()`（默认无
  总超时）——提供商挂起时会议永久卡在 processing 且前端单飞锁不释放。转写 600s /
  摘要 300s / 同步 30s。
- **[P2] 详情视图乱序响应竞态**：快速切换会议 A→B 时迟到的 A 响应覆盖 B 视图（Delete
  按钮随后作用于错误会议）。渲染前校验 `currentMeetingId === id`，过期响应丢弃。

### Changed — tests
- Go：新增 `/sync` 租户隔离回归（`sync_ownership_test.go`）与队列 worker 集成测试
  （`worker_pg_test.go`，`RECAP_TEST_DSN` 门控：领取/终态/退避三断言）。
- Rust：`cargo test` 15/15；`cargo check/build` 全过（含新 sync 模块与命令）。

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
