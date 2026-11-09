# rich_text · 会话初始化（跨工具兜底）

> 协同规约正文见全局 `~/.agents/skills/dev-loop/SKILL.md` §3「会话绑定」；本文件只放启动胶水，禁复制机制正文。
> 固定单 IDE 且自动路由正常时可整节删除；跨多工具使用时保留本兜底。

## 会话初始化（新窗口开场执行一次）

- 开场含「继续/开工/初始化」或首次指令时：先跑 `toolbox run-hooks bootstrap --quiet`（非 0 仅附一行 ⚠，不阻塞；无 toolbox 跳过）→ 跑 `toolbox run panel --root .`（无 toolbox 则读 `context/CURRENT` → 绑定 `context/epics/misc/memory.md` 断点）→ 输出会话绑定卡（域/类型/挂载/任务全景/状态）→ 按断点开工或等指令；多任务切换 `::board`（绑定纪律与恢复规约归 dev-loop §3）。
- `context/CURRENT` 缺失或为 `none` → 列 `context/epics/` 候选请用户选，严禁自选开工；无 `context/` → 提示走 dev-init 接入；命令速查 `~/.agents/COMMANDS.md`。

## 项目事实速查

- 纯 Rust 原生 SDK（方案 B）：`native/rich_text_core`（唯一事实源，零运行时依赖）+ `native/rich_text_uniffi`（UniFFI 绑定层，proc-macro 模式）。
- 工作流：Rust 工具链；`cd native/rich_text_core && cargo test` / `cd native/rich_text_uniffi && cargo build`；`uniffi-bindgen generate --library <cdylib> --language <kotlin|swift>` 产出各端桩。
- 设计 SSOT 在 goodshare 仓 `docs/design/`；本仓 `docs/` 仅放评估类文档；黄金语料在 `native/rich_text_core/tests/`。
