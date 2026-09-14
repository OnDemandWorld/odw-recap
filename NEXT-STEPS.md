# ODW Recap — 下一步开发计划 (Next Steps)

> 生成时间：2026-09-13。基于对 `dev` 分支当前代码的完整 Review + 全量测试得出。
> 每项都标注了**涉及文件**、**验收标准 (DoD)** 与**工作量**（S/M/L = 约 0.5天 / 1-3天 / >3天）。
> 优先级：P0 阻断核心承诺 → P1 功能完整度 → P2 运维/规模 → P3 工程化。

---

## 当前状态快照（先说清楚"已经能跑什么"）

- **桌面端**：导入（文件/拖拽/监听文件夹/手机上传）→ 云端转写（OpenAI/Deepgram）
  → 摘要（OpenAI/Anthropic，离线回退规则摘要）→ 转写/摘要/行动项/决策**已真正落库并在 UI 展示**，
  支持查看与删除。15 个 Rust 测试 + 22 个 JS 测试通过。
- **team-server**：JWT 注册/登录（含限流、请求体上限、输入校验）、RBAC、会议 CRUD（租户隔离）、
  审计日志、Recap→Vault/Loop 转发，全部经真实 PostgreSQL 冒烟验证。

下面的"缺口"都是在此之上的增量，**不是"现在不能用"**。

---

## P0 — 兑现"on-device"核心承诺（当前依赖云 provider）

### 1. 本地转写引擎接入（whisper.cpp）
- **现状**：`transcription/providers/whisper_local.rs` 是显式 stub，选择它处理会返回
  "not available yet" 错误。
- **要做的**：用 `whisper-rs`（whisper.cpp 的 Rust 绑定）实现真正的本地推理；
  支持模型下载/缓存（ggml 格式，建议默认 small/medium）；实现 `STTProvider::transcribe`，
  产出带时间戳的 `TranscriptSegment`。
- **DoD**：一台离线 Mac 上，选择 "Whisper Local" + 不填任何 API Key，能完成导入→转写→
  规则摘要全流程；`cargo test` 增加对模型加载与一段短音频（可用测试 fixture）的转写断言。
- **工作量**：L（含模型分发策略）。

### 2. 本地摘要引擎接入（llama.cpp / Ollama）
- **现状**：`llama_local.rs` 是 stub；`ollama.rs` 需本地跑 Ollama 服务（半可用）。
- **要做的**：`llama_local.rs` 用 `llama-cpp-2`/`llama-cpp-rs` 绑定实现，或明确把
  Ollama 作为唯一本地路径并实现"自动检测 11434 端口 + 拉取模型"引导。二选一，避免两个都半残。
- **DoD**：无云 Key 时可生成非规则摘要的结构化 summary；UI 提示模型来源。
- **工作量**：L。

### 3. 真实音频采集（麦克风 + 系统内录）
- **现状**：`audio_capture/recorder.rs`、`system_audio.rs` 返回假设备列表，无实际 I/O。
  UI 顶部"Meeting Library"承诺了录制能力但无入口。
- **要做的**：引入 `cpal` 实现麦克风录制；系统内录分平台
  （macOS ScreenCaptureKit/BlackHole、Windows WASAPI loopback、Linux PulseAudio/PipeWire）。
  录制产物写入 inbox 并建会议。
- **DoD**：能录一段真实音频、导入会议库、成功转写。
- **工作量**：L（跨平台 loopback 尤其大）。

> 说明：PRD 把"no cloud bot / on-device"作为卖点，以上 1-3 是把它从"文档承诺"变成"可交付能力"的关键。

---

## P1 — 已存在但"半成品"的功能（这些改完产品就完整）

### 4. 全文检索（Transcript Search）——目前是完全的死功能
- **现状**：`sqlite_manager.rs` 定义了 FTS5 外部内容表 `transcript_search` 和
  `search_transcripts()`，但：① 保存转写片段时**从不写入** FTS 表（外部内容表需手动同步）；
  ② 没有任何 `#[tauri::command]` 暴露它；③ 前端无搜索框。整条链路是断的。
- **要做的**：在 `save_transcript_segments` 事务内同步维护 `transcript_search`
  （或加 SQLite `AFTER INSERT/DELETE` 触发器）；新增 `search_transcripts` command + 前端搜索框。
- **DoD**：会议库顶部输入关键词，能返回命中会议与片段；单元测试覆盖写入+检索。
- **工作量**：S/M。

### 5. 提示词模板真正参与摘要生成
- **现状**：模板能存/列（`save_prompt_template`/`list_prompt_templates`），
  但 `processing.rs` 的 `summarize()` 从不读取模板（`prompt_template_used` 恒为 None），
  `prompt_manager` 模块整体是死代码。
- **要做的**：`process_meeting` 读取配置里选定的模板，用 `variable_substitution`
  把 `{{transcript}}`/`{{topic}}` 等注入 LLM 请求；UI 增加"为会议选择模板"。
- **DoD**：选择不同模板产出不同风格摘要，`prompt_template_used` 落库正确。
- **工作量**：M。

### 6. 监听文件夹：路径持久化 + 重启自动恢复
- **现状**：`start_watch_folder` 只接受运行时路径，选定的文件夹**不写入 config、重启不恢复**
  （`config/mod.rs` 已有 `watch_folder_enabled`/`watch_folder_path` 字段但命令没用它）。
- **要做的**：`start_watch_folder` 落库路径与 enabled 标记；App 启动时若 enabled 则自动重新挂载。
- **DoD**：设置一次，重启后仍在监听；UI 显示当前监听状态。
- **工作量**：S。

### 7. 桌面端 → team-server 同步（SyncEngine 落地）
- **现状**：`sync/client.rs`、`sync/mod.rs` 是空壳（`sync_to_vault`/`sync_to_loop` 直接
  `Ok(())`），`sync_queue` 表建了但没人写。桌面端与 Go 服务器之间**没有任何真实数据通道**，
  团队多用户功能名存实亡。
- **要做的**：实现 `SyncClient` 对 team-server `/auth/login`、`/meetings`、`/sync` 的真实调用
  （带 JWT），桌面端加"登录团队账号 + 同步"命令与 UI；失败入队重试（复用 `sync_queue`）。
- **DoD**：桌面端登录后点同步，会议出现在 team-server `/meetings`，`audit_log` 有记录。
- **工作量**：M。

### 8. 其余云 provider 补齐（AssemblyAI / Azure / Google / AWS / Gemini / Bedrock）
- **现状**：这些 provider 均为返回固定文本的 stub。
- **要做的**：参照已实现的 `openai_whisper.rs`/`deepgram.rs` 补齐 REST 客户端。
- **DoD**：每个 provider 有可用实现 + provider 单测（用 mock HTTP）。
- **工作量**：M（可拆成多个并行小任务）。

---

## P2 — team-server 规模化与账号安全

### 9. sync_queue 消费端（worker）
- **现状**：`/sync` 只写一行 `pending` 到 `sync_queue`，没有任何消费者，队列只增不减。
  跨产品转发目前是**请求内同步完成**的，队列仅作审计。
- **要做的**：要么加后台 worker 按 `status='pending'` 消费+重试+退避+死信；要么
  明确放弃队列、由请求内转发兜底并从 schema 移除 `sync_queue` 的误导。二选一。
- **工作量**：M（选做 worker）。

### 10. 限流器多实例化（当前内存级，仅单实例有效）
- **现状**：`api/ratelimit.go` 是进程内固定窗口；多副本部署时限流失效。
- **要做的**：改用 Redis（`db/redis.go` 已接入但**全项目零调用**，是死代码）实现分布式限流。
- **工作量**：S/M。

### 11. 账号生命周期：改密 / 登出吊销 / 刷新令牌 / 找回
- **现状**：JWT 固定 24h、无刷新、无吊销（角色降级后旧 token 仍有效）；无改密/找回接口。
- **要做的**：加 `/auth/refresh`、`/auth/logout`（Redis 黑名单或短期 access + 长 refresh）、
  改密与邮箱找回流程；角色变更时使 token 版本失效。
- **工作量**：M。

### 12. 组织管理 API（organizations / members）
- **现状**：建了表、`models` 有结构，但无任何组织相关 endpoint，`OrganizationID`
  在 `Meeting` 里是非指针 `uuid.UUID`（列可空 → 潜在扫描隐患）。
- **要做的**：org CRUD + 成员管理 + 会议归属组织；`OrganizationID` 改 `*uuid.UUID`。
- **工作量**：M。

---

## P3 — 工程化 / 交付 / 测试覆盖

### 13. CI 补齐：真实依赖的集成测试
- **现状**：GitHub Actions 只跑 `cargo test` + `go test ./...`（纯单测）；
  team-server 的 register/login/db store/迁移路径、桌面端流水线**没有针对真实 PostgreSQL 的测试**。
- **要做的**：CI 加 `services: postgres/redis`，为 `dbMeetingStore`、auth handler 写
  `TEST_DATABASE_URL` 门控的集成测试；可选用 `sqlmock` 做无 DB 快测。
- **工作量**：S/M。

### 14. 桌面端打包与分发
- **要做的**：macOS codesign + notarytool 公证、Windows 代码签名、`tauri.conf.json`
  配置 updater（endpoint + 公钥），否则用户拿到的包被 Gatekeeper/SmartScreen 拦截。
- **工作量**：M（依赖拿到签名证书/开发者账号）。

### 15. 可观测性
- **要做的**：team-server 结构化日志（slog/zap）、请求指标、`/health` 增加 DB/Redis 连通性探测
  （现在恒返回 ok，不能反映依赖故障）。
- **工作量**：S/M。

### 16. 前端测试与 Tauri 2.x
- **要做的**：桌面 E2E 目前是占位（需 `@playwright/test` + 已构建 app，见
  `desktop-app/tests/e2e/`），补最小冒烟；评估 Tauri 1.4 → 2.x 迁移（权限模型、IPC 变化）。
- **工作量**：M。

---

## 建议的推进顺序（一条可执行的路线）

| 顺序 | 任务 | 理由 |
|---|---|---|
| 1 | #4 全文检索、#6 watch folder 持久化、#5 模板接入 | 都在已接通的流水线上，投入小、体验提升明显 |
| 2 | #7 桌面↔server 同步 | 打通"个人工具 → 团队产品"的主干 |
| 3 | #1 本地 whisper、#2 本地 llama | 兑现 on-device 核心卖点（最大工程量） |
| 4 | #3 真实录音采集 | 补齐"录制"入口 |
| 5 | #8 云 provider 补齐、#9-12 规模化/账号 | 面向企业交付 |
| 6 | #13-16 CI/签名/可观测/E2E | 发布就绪度 |

---

## 附：本次 Review 已修复（不在待办内，供对照）

远程 Google Fonts → 本地自托管并收紧 CSP、拖拽多文件导入、team-server 审计 `details`
落库、`sync_queue` 外键回退、`main.go` gofmt。测试：Rust 15 / JS 22 / Go 全绿，
并用真实 PostgreSQL + 运行中的 Vault 完成了端到端冒烟。详见 [CHANGELOG.md](CHANGELOG.md)。
