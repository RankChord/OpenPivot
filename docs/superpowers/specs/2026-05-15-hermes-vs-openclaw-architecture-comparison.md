# Hermes Agent vs OpenClaw 架构设计对比分析

> 分析日期: 2026-05-15
> 版本: v1.0

## 目录

1. [项目概述](#1-项目概述)
2. [核心架构](#2-核心架构)
3. [代理循环设计](#3-代理循环设计)
4. [工具系统](#4-工具系统)
5. [插件/扩展系统](#5-插件扩展系统)
6. [技能系统](#6-技能系统)
7. [记忆系统](#7-记忆系统)
8. [Gateway 架构](#8-gateway-架构)
9. [通道/平台适配器](#9-通道平台适配器)
10. [会话管理](#10-会话管理)
11. [委派/子代理系统](#11-委派子代理系统)
12. [定时任务系统](#12-定时任务系统)
13. [关键设计决策对比](#13-关键设计决策对比)
14. [设计哲学对比](#14-设计哲学对比)
15. [技术栈对比](#15-技术栈对比)
16. [测试策略](#16-测试策略)
17. [安全模型对比](#17-安全模型对比)
18. [总结与建议](#18-总结与建议)

---

## 1. 项目概述

### Hermes Agent

**定位**: 自进化 AI 代理（Self-improving AI Agent）
**组织**: Nous Research
**语言**: Python 3.11+
**许可**: MIT
**规模**: ~3,434 文件，核心文件 `run_agent.py` 达 ~16,000 LOC

**核心特性**:
- 内置学习循环，从经验中创建技能
- 会话中自我改进技能
- 跨会话的用户画像建模（Honcho 方言建模）
- 支持 40+ 内置工具和自定义工具集
- 7 种终端后端（local, Docker, SSH, Singularity, Modal, Daytona, Vercel Sandbox）
- 7+ 消息平台网关（Telegram, Discord, Slack, WhatsApp, Signal, 等）
- TUI 界面（React/Ink 前端 + Python JSON-RPC 后端）
- 研究就绪：批量轨迹生成、Atropos RL 环境、轨迹压缩

### OpenClaw

**定位**: 个人 AI 助手（Personal AI Assistant）
**组织**: 个人项目（为 Molty 设计）
**语言**: TypeScript (Node 22+)
**许可**: MIT
**规模**: ~17,584 文件，monorepo 结构

**核心特性**:
- 本地优先的 Gateway 控制平面
- 多通道消息收件箱（25+ 平台支持）
- 多代理路由
- 语音唤醒 + 语音模式（macOS/iOS/Android）
- Live Canvas 可视化工作区
- CLI 控制 + Companion 原生应用
- Clawhub 技能市场
- 沙箱执行支持

## 2. 核心架构

### Hermes Agent: 单体核心架构

```
hermes-agent/
├── run_agent.py          # AIAgent 类 — 核心对话循环 (~16k LOC)
├── model_tools.py        # 工具编排、discover_builtin_tools()、handle_function_call()
├── toolsets.py           # 工具集定义，_HERMES_CORE_TOOLS 列表
├── cli.py                # HermesCLI 类 — 交互式 CLI 编排 (~11k LOC)
├── hermes_state.py       # SessionDB — SQLite 会话存储 (FTS5 搜索)
├── hermes_constants.py   # get_hermes_home() — 配置文件感知路径
├── hermes_logging.py     # setup_logging() — 日志系统
├── agent/                # 代理内部（provider 适配器、memory、缓存、压缩等）
├── hermes_cli/           # CLI 子命令、设置向导、插件加载器、皮肤引擎
├── tools/                # 工具实现 — 通过 tools/registry.py 自动发现
├── gateway/              # 消息网关 — run.py + session.py + platforms/
├── plugins/              # 插件系统
├── skills/               # 内置技能
├── ui-tui/               # Ink (React) 终端 UI
├── tui_gateway/          # Python JSON-RPC TUI 后端
├── cron/                 # 调度器 — jobs.py, scheduler.py
└── tests/                # Pytest 测试套件 (~17k tests, ~900 files)
```

**架构特点**:
- **单体核心**: `run_agent.py` 包含 ~16,000 行代码的 `AIAgent` 类，这是整个系统的核心
- **模块导入链**: `tools/registry.py` → `tools/*.py` → `model_tools.py` → `run_agent.py`/`cli.py`
- **配置文件隔离**: `HERMES_HOME` 环境变量控制所有状态路径（`~/.hermes/`），支持 profiles
- **异步桥接**: 三层方案处理 Python 异步/同步混合（持久事件循环、工作线程、网关/RL）

### OpenClaw: Monorepo 模块化架构

```
openclaw/
├── src/                  # 核心 TypeScript 代码
│   ├── agents/           # 代理逻辑 (~922 files)
│   ├── gateway/          # Gateway 服务器 (~485 files)
│   ├── channels/         # 通道注册和路由
│   ├── tools/            # 工具实现
│   ├── plugins/          # 插件加载器和运行时
│   ├── sessions/         # 会话管理
│   ├── config/           # 配置处理
│   └── plugin-sdk/       # 插件 SDK 边界 (~481 files)
├── ui/                   # Control UI
├── packages/             # 共享包 (SDK, plugin-sdk)
├── extensions/           # 134 个插件实现
├── apps/                 # 原生应用（macOS, iOS, Android）
└── docs/                 # 文档
```

**架构特点**:
- **Monorepo**: pnpm workspace 结构，`src/`、`ui/`、`packages/`、`extensions/` 独立包
- **严格的插件边界**: 核心通过 `openclaw/plugin-sdk/*` 与插件交互，插件不能直接访问核心源码
- **惰性加载**: 强调启动性能，按需加载模块（`*.runtime.ts` 文件）
- **动态导入**: 使用 `*.runtime.ts` 文件作为惰性边界，避免在启动时加载不必要的代码

## 3. 代理循环设计

### Hermes Agent: 同步循环

**文件**: `run_agent.py:11828` (`run_conversation()` 方法)

```
while (api_call_count < self.max_iterations and self.iteration_budget.remaining > 0) \
        or self._budget_grace_call:
    1. 检查中断请求
    2. Memory 预取（后台召回）
    3. 处理 /steer 引导
    4. 构建 api_messages（注入插件上下文、清理代理、应用提示缓存）
    5. 调用 LLM API（带重试和退避策略）
    6. 解析响应:
       - 如果有 tool_calls → 执行工具（顺序或并发），追加结果到消息
       - 如果是文本响应 → 返回最终答案
    7. 回合后：同步 memory，排队下次预取
```

**关键机制**:
- **预算系统**: `IterationBudget`（默认 90），父代理和子代理共享
- **工具执行分支**: 根据工具名称决定顺序或并行执行
  - 只读工具（`_PARALLEL_SAFE_TOOLS`）可并行（最多 8 workers）
  - 写操作工具强制顺序执行
  - `clarify` 等工具强制顺序模式
- **Grace 调用**: 预算耗尽后给予一回合总结机会
- **传输层适配器**: 支持多种 API 模式（chat_completions, anthropic_messages, codex_responses, bedrock_converse, codex_app_server）

### OpenClaw: 嵌入式流式代理

**文件**: `src/agents/pi-embedded-runner.ts` 和 `src/agents/pi-embedded-runner/run.js` (`runEmbeddedPiAgent`)

OpenClaw 的代理循环设计为**嵌入式流式模式**:

```
runEmbeddedPiAgent(session)
  1. 构建系统提示（缓存优先）
  2. 加载 workspace 引导文件（AGENTS.md, SOUL.md, TOOLS.md）
  3. 解析工具目录（核心工具 + 插件工具）
  4. 启动流式 LLM 调用
  5. 处理响应:
     - 文本块 → 实时流式传输到客户端
     - 工具调用 → 分发给工具执行器
     - 工具结果 → 注入消息继续循环
  6. 支持流式中断和恢复
```

**关键机制**:
- **流式优先**: 所有响应都是流式的，支持实时 token 传输
- **会话持久化**: 使用文件系统存储会话转录和状态
- **Auth Profile 轮换**: 支持多个 auth profile 的自动故障转移
- **上下文窗口保护**: 自动压缩和摘要过长上下文
- **Compaction**: 中间对话轮次使用辅助模型（廉价）自动摘要

## 4. 工具系统

### Hermes Agent: 注册表 + 工具集

**注册表**: `tools/registry.py`

```python
class ToolEntry:
    name: str
    toolset: str           # 所属工具集
    schema: dict           # OpenAI 格式工具 schema
    handler: callable      # 工具处理函数
    check_fn: callable     # 可用性检查（环境/依赖）
    requires_env: list     # 需要的环境变量
```

**自动发现流程**:
1. `discover_builtin_tools()` 扫描 `tools/*.py`
2. 使用 AST 查找有顶层 `registry.register()` 调用的文件
3. 导入这些文件，触发注册

**工具集系统** (`toolsets.py`):
- `_HERMES_CORE_TOOLS` = 基线工具集
- 每个平台（telegram, discord, cli）继承基线并可能添加平台特定工具
- 通过 `hermes tools`（curses UI）启用/禁用每个平台的工具

**Agent 级工具** (`model_tools.py:493`):
- `todo`, `memory`, `session_search`, `delegate_task` 被代理循环拦截
- 这些需要访问代理级状态（TodoStore, MemoryStore 等）

### OpenClaw: 插件工具 + 核心工具

**注册方式**:
- 核心工具定义在 `src/tools/`
- 插件工具通过插件系统的工具注册机制注册
- 使用 `pi-tools.ts` 和 `openclaw-tools.ts` 进行工具交付

**工具策略**:
- **Sandbox 策略**: 工具可以在沙箱中执行
- **Owner 授权**: 某些工具需要 owner 授权
- **Visibility 策略**: 工具对不同会话可见性不同
- **Schema 兼容**: 处理不同 provider 的工具 schema 兼容性

**关键设计**:
- 工具执行结果分类和错误处理
- 工具目录支持动态发现和懒加载
- 工具执行策略支持并发控制

## 5. 插件/扩展系统

### Hermes Agent: 多层插件架构

**发现来源**（顺序 = 后来覆盖前面）:
1. 捆绑插件: `<repo>/plugins/<name>/`
2. 用户插件: `~/.hermes/plugins/<name>/`
3. 项目插件: `./.hermes/plugins/<name>/`（由 `HERMES_ENABLE_PROJECT_PLUGINS` 控制）
4. Pip entry points: `hermes_agent.plugins` 组

**插件类型**:
- `standalone`（默认）— hooks/tools，通过 `plugins.enabled` 选择加入
- `backend` — 可插拔后端（如 image_gen），自动加载
- `exclusive` — 恰好一个活动提供者（memory），通过配置选择
- `platform` — 网关适配器，捆绑自动加载
- `model-provider` — 推理后端，通过 `providers/` 单独发现

**生命周期钩子**:
```python
VALID_HOOKS = {
    "pre_tool_call", "post_tool_call", "transform_tool_result",
    "transform_llm_output", "transform_terminal_output",
    "pre_llm_call", "post_llm_call",
    "on_session_start", "on_session_end", "on_session_finalize",
    "on_session_reset", "subagent_stop",
    "pre_approval_request", "post_approval_response",
}
```

**PluginContext API**:
- `register_tool()`, `register_hook()`, `register_cli_command()`
- `register_command()`, `register_platform()`, `register_context_engine()`
- `register_image_gen_provider()`, `register_skill()`
- `inject_message()`, `dispatch_tool()`, `ctx.llm`

### OpenClaw: 严格 SDK 边界插件

**架构**:
- 核心保持插件无关（plugin-agnostic）
- 插件只能通过 `openclaw/plugin-sdk/*` 与核心交叉
- 插件生产代码：不能访问核心 `src/**` 或其他插件 `src/**`
- 外部插件通过 `facade-runtime` 或通用合约访问

**插件加载**:
- 捆绑插件在核心 dist 中交付
- 外部官方插件拥有自己的包/依赖，从核心 dist 中排除
- 使用 `facade-loader.ts` 和 `facade-runtime.ts` 进行惰性加载

**关键原则**:
- **Owner 边界**: owner 特定的修复/检测/引导/认证/默认/provider 行为位于 owner 插件中
- **热路径优化**: 准备的事实向前携带（provider id, model ref, channel id 等）
- **向后兼容**: 所有新接缝向后兼容、有文档、有版本
- **延迟加载**: 使用 `api.runtime` 或专注的 SDK facade，避免在启动时加载 broad barrel

## 6. 技能系统

### Hermes Agent: SKILL.md 基于目录

**组织结构**:
- `skills/` — 捆绑技能（25 个分类），默认加载
- `optional-skills/` — 较重的/小众技能，默认不活动
- 插件技能通过 `ctx.register_skill()` 注册，名称空间化为 `<plugin>:<skill>`

**SKILL.md frontmatter**:
```yaml
name: My Skill
description: 短描述（≤ 60 字符）
version: 1.0.0
author: Human Name (@github)
platforms: [linux, macos]    # 可选 OS-gating
metadata:
  hermes:
    tags: [dev, tools]
    category: developer
    related_skills: [other-skill]
    config:                   # 需要的 config.yaml 设置
```

**技能加载机制**:
- 技能作为上下文加载，而非代码执行
- 提供 instructions、scripts (`scripts/`)、reference materials (`references/`, `templates/`)
- 作为斜杠命令 (`/<skill-name>`) 出现，列在系统提示中

**Curator 系统**: `agent/curator.py` 自动归档由 agent 创建的过时技能，pinned 技能豁免

### OpenClaw: Skills via Clawhub

**组织结构**:
- 技能通过 `skills/` 目录和 Clawhub 市场管理
- 技能通过 SKILL.md frontmatter 定义
- 支持 bundled 技能和 workspace 技能

**技能安装**:
- `openclaw skills install` 从 Clawhub 安装
- 支持 workspace 级别的技能
- 技能状态管理和归档

## 7. 记忆系统

### Hermes Agent: 多提供者 Memory 架构

**架构**: `agent/memory_manager.py` 协调提供者，`agent/memory_provider.py` 定义 ABC

**MemoryProvider ABC 生命周期**:
- `is_available()` — 检查凭证/依赖
- `initialize(session_id)` — 连接、预热
- `system_prompt_block()` — 静态系统提示文本
- `prefetch(query)` — 召回下一轮的上下文
- `sync_turn(user, assistant)` — 持久化完成的回合
- `shutdown()` — 清理

**内置提供者** (`plugins/memory/`):
- honcho, mem0, supermemory, byterover, hindsight, holographic, openviking, retaindb

**关键设计**:
- 最多 ONE 外部提供者（防止工具 schema 膨胀）
- 记忆上下文包装在 `<memory-context>` XML 围栏中
- 通过 `StreamingContextScrubber` 从流式输出中清除
- 支持 Honcho dialectic 用户建模

### OpenClaw: 基于文件的 Mark Memory

**架构**:
- 使用文件系统存储记忆
- 支持 `MEMORY.md` 和 `USER.md` 文件
- 基于文件的持久化，支持跨会话回忆
- 支持 memory host SDK 和多种记忆后端

**关键设计**:
- 简单、文件优先的记忆模型
- 通过 workspace 目录结构组织
- 支持嵌入记忆和外部记忆提供者

## 8. Gateway 架构

### Hermes Agent: 平台适配器 Gateway

**Gateway Runner** (`gateway/run.py`):

`GatewayRunner` 是管理平台适配器生命周期主控进程。

**消息流**:
1. 平台适配器接收消息 → 创建 `MessageEvent`（归一化于 `gateway/platforms/base.py`）
2. `GatewayRunner._process_message_background()` 处理路由
3. 命令拦截（`/stop`, `/new`, `/reset` 等）于 `gateway/run.py`
4. Agent 实例创建/获取（LRU Cache of 128, 1h idle TTL）
5. `AIAgent.run_conversation()` 执行
6. 响应通过适配器传回

**会话管理** (`gateway/session.py`):
- SessionStore：跟踪每个平台适配器的活动会话
- SessionKey 格式：`agent:main:{platform}:{chat_type}:{chat_id}[:{extra}]`
- SessionContext：携带平台、user_id、聊天元数据
- 通过 scoped locks 实现 profile 隔离

**关键架构决策**:
- 每个平台适配器有自己的 `_active_sessions` dict，防止同一会话的并发处理
- `EphemeralReply` 包装字符串带有自动删除 TTL
- 图像/音频/视频/文档缓存在 `HERMES_HOME/cache/`，自动清理
- Gateway 配置桥接从 `config.yaml` 到 env vars 在模块加载时：`gateway/run.py:389-577`

### OpenClaw: WebSocket/HTTP 协议 Gateway

**Gateway Server** (`src/gateway/server.ts` / `src/gateway/server.impl.ts`):

- 支持 WebSocket 和 HTTP 协议
- 控制平面审计和速率限制
- 设备配对和节点管理
- 会话管理和转录持久化

**关键组件**:
- `server-chat.ts` — 聊天处理
- `server-channels.ts` — 通道处理
- `server-sessions.ts` — 会话管理
- `server-plugins.ts` — 插件生命周期
- `server-cron.ts` — 定时任务
- `server-control-ui.ts` — 控制 UI

**协议设计**:
- 版本化 Gateway 协议 (`src/gateway/protocol/`)
- 开放/响应 HTTP 兼容
- MCP HTTP 支持
- Tailscale 集成支持

## 9. 通道/平台适配器

### Hermes Agent: BasePlatformAdapter ABC

**平台适配器模式** (`gateway/platforms/base.py:1265`):

`BasePlatformAdapter` ABC 定义接口。所有 20+ 适配器继承自它：
- Telegram, Discord, WhatsApp, Slack, Signal, Matrix, Mattermost
- DingTalk, WeCom, Weixin, QQBot, Feishu, Yuanbao
- Email, SMS, Webhook, HomeAssistant, API Server

**必需实现**:
- `connect()` / `disconnect()`
- `start_listening()`
- `send(chat_id, text, ...)` → `SendResult`
- `edit_message(...)`, `delete_message(...)`
- 可选：流式草案支持、媒体处理、打字指示器

**平台注册** (`gateway/platform_registry.py`):
- 声明式平台注册系统
- 适配器实现注册，自动发现和加载

### OpenClaw: Channel 插件系统

**通道架构** (`src/channels/`):
- 通道作为插件实现
- 严格的通道合约（`ChannelPlugin`, `ChannelMessagingAdapter`, `ChannelOutboundAdapter`）
- 支持轮询和推送模式
- 通道路由和会话绑定

**通道类型**:
- 消息平台：WhatsApp, Telegram, Slack, Discord, Signal, iMessage, IRC, Teams, Matrix, 等
- 设备/节点：macOS, iOS, Android 节点
- 自定义：WebChat, Webhook, Nostr

**关键设计**:
- 通道配置和路由在 `src/config/` 中定义
- 支持多代理路由（不同通道/账户/对等方路由到隔离代理）
- 线程绑定和会话管理
- 允许从/ DM 策略和配对机制

## 10. 会话管理

### Hermes Agent: SQLite SessionDB

**会话存储**: `hermes_state.py` (`SessionDB`)

- SQLite 数据库存储会话和消息
- FTS5 全文搜索支持
- 会话键格式：`agent:main:{platform}:{chat_type}:{chat_id}[:{extra}]`
- 支持会话压缩和摘要
- 跨会话搜索和回忆

**会话状态**:
- 会话上下文携带平台、用户、聊天元数据
- 支持会话恢复和分支
- 会话与 Gateway 会话解藕，cron 会话不镜像到目标 Gateway 会话

### OpenClaw: 文件会话系统

**会话存储**:
- 使用文件系统存储会话转录和状态
- 会话目录结构组织
- 支持会话压缩和恢复
- 会话隔离和路由

**会话键**:
- 基于通道和目标的会话键
- 支持线程绑定和会话绑定
- 会话状态持久化和恢复

## 11. 委派/子代理系统

### Hermes Agent: delegate_task 工具

**委派工具** (`tools/delegate_tool.py`):

- 生成具有隔离上下文 + 终端会话的子代理
- 同步：父代理等待子代理摘要完成
- 两种形式：
  - **Single**: 传递 `goal`（+ 可选 `context`, `toolsets`）
  - **Batch（并行）**: 传递 `tasks: [...]` — 每个都有自己的子代理并发运行
  - 并发上限由 `delegation.max_concurrent_children`（默认 3）

**角色**:
- `role="leaf"`（默认）— 专注的 worker。无法调用 `delegate_task`, `clarify`, `memory`, `send_message`, `execute_code`
- `role="orchestrator"` — 保留 `delegate_task` 所以它可以生成自己的 worker。由 `delegation.orchestrator_enabled` 和 `delegation.max_spawn_depth`（默认 2）门控

**关键配置** (`config.yaml` 中的 `delegation:`):
- `max_concurrent_children`, `max_spawn_depth`, `child_timeout_seconds`
- `orchestrator_enabled`, `subagent_auto_approve`, `inherit_mcp_toolsets`

### OpenClaw: 子代理注册系统

**子代理架构** (`src/agents/` 中的 `subagent-*` 文件):

- 子代理注册表和生命周期管理
- 子代理生成和上下文传递
- 支持嵌套子代理
- 子代理公告和交付
- 子代理会话清理和恢复

**关键设计**:
- 子代理深度限制（防止无限递归）
- 子代理会话键和度量
- 子代理系统提示和初始用户消息
- 子代理目标策略和任务名称

## 12. 定时任务系统

### Hermes Agent: Cron 调度器

**Cron 架构** (`cron/jobs.py` + `cron/scheduler.py`):

- 支持多种调度格式：
  - 持续时间：`"30m"`, `"2h"`, `"1d"`
  - "every" 短语：`"every 2h"`, `"every monday 9am"`
  - 5-field cron 表达式：`"0 9 * * *"`
  - ISO 时间戳（一次性）：`"2026-06-01T09:00:00Z"`

**关键加固**:
- **3 分钟硬中断** — 长时间运行的代理循环无法垄断调度器
- Catchup 窗口：作业周期的一半，限制在 120s-2h
- Grace 窗口：120s 用于错过触发时间的一次性作业
- 文件锁在 `~/.hermes/cron/.tick.lock` 防止多进程重复 tick
- Cron 会话默认传递 `skip_memory=True`

### OpenClaw: Gateway 集成 Cron

**Cron 架构** (`src/gateway/server-cron.ts` / `src/cron/`):

- 与 Gateway 服务器集成的定时任务系统
- 支持定时作业和周期作业
- 通过 Gateway 协议管理
- 支持作业通知和交付

## 13. 关键设计决策对比

### Hermes Agent

| 决策 | 描述 |
|------|------|
| **系统提示不可变性** | 每个会话构建一次系统提示（`_cached_system_prompt`）并逐字重用。插件上下文从不注入系统提示 — 而是进入用户消息。这保留了 Anthropic 前缀缓存和 OpenAI 提示缓存，将输入成本降低约 75% |
| **工具定义记忆化** | `get_tool_definitions()` 在 `(enabled_toolsets, disabled_toolsets, registry_generation, config_mtime)` 上记忆。注册表有 `_generation` 计数器，每次突变时跳动，所以 MCP 刷新或插件加载自动使缓存无效 |
| **安全工具执行** | 注册表捕获 ALL 异常并返回 `{"error": "..."}` JSON — 工具永远不会崩溃代理循环。错误消息被 `agent/error_classifier.py` 分类 |
| **基于 profile 的隔离** | 所有状态路径使用 `get_hermes_home()`（从不硬编码 `~/.hermes`）。每个 profile 都有自己的配置、API 密钥、记忆、会话、技能和 gateway |
| **上下文压缩** | 使用辅助（便宜）模型自动摘要中间对话回合。保护头部和尾部上下文，跟踪已解决/待决问题，摘要预算与压缩内容成比例缩放 |
| **中断模型** | 使用线程级信号（`_set_interrupt()`）。子代理中断显式扇出到工作线程。`/steer` 提供非中断引导注入 |
| **JSON 修复管道** | 处理来自开放权重模型的格式错误工具调用 JSON。多次修复传递，回退到空对象 |
| **插件不可变核心** | 插件**不得**修改核心文件。如果插件需要框架未暴露的能力，必须通用扩展插件表面 |
| **记忆提供者专一性** | 一次只能有一个外部记忆提供者。这防止工具 schema 膨胀和冲突后端 |

### OpenClaw

| 决策 | 描述 |
|------|------|
| **核心插件无关** | 核心通过 `plugin-sdk` 与插件交互，不直接依赖插件实现。manifest/registry/能力合约工作 |
| **惰性加载** | 强调启动性能。使用 `*.runtime.ts` 文件作为惰性边界，避免在启动时加载不必要的代码 |
| **Owner 边界** | owner 特定的行为位于 owner 插件中。共享/核心只获得通用接缝 |
| **向后兼容** | 所有新接缝向后兼容、有文档、有版本。第三方插件存在 |
| **热路径优化** | 准备的事实向前携带（provider id, model ref, channel id 等）。不使用 broad 加载器重新发现 |
| **Gateway 协议版本化** | Gateway 协议变更首先 additive。不兼容需要版本化/文档/客户端后续 |
| **插件 SDK 边界** | 插件通过 SDK 访问核心能力，不能直接访问核心源码。SDK 子路径帮助调用者一次解决一个能力 |
| **配置合同** | 导出类型、schema/help、元数据、基线、文档对齐。退役的公共键保持退役 |

## 14. 设计哲学对比

### Hermes Agent: 自进化代理

**核心哲学**:
- **学习循环**: 代理从经验中学习，创建和改进技能
- **知识持久性**: 技能自我改进，用户画像跨会话深化
- **灵活性**: 支持多种模型提供者、平台、工具
- **研究就绪**: 内置 RL 训练、轨迹生成和压缩
- **可运行在任何地方**: $5 VPS、GPU 集群或 serverless 基础设施

**设计模式**:
- Python 同步核心，异步桥接
- 多层插件架构
- 基于目录的技能系统
- 基于注册表的工具系统
- Profile 隔离

### OpenClaw: 个人 AI 助手

**核心哲学**:
- **本地优先**: Gateway 只是控制平面 — 产品是助手
- **插件边界**: 核心保持插件无关
- **性能**: 惰性加载、热路径优化、启动性能
- **多通道**: 支持 25+ 平台，统一的助手体验
- **原生应用**: macOS、iOS、Android 伴侣应用

**设计模式**:
- TypeScript ESM，严格类型
- Monorepo（pnpm workspace）
- 严格 SDK 边
- 文件优先存储（会话、记忆、配置）
- 流式传输和实时交互

## 15. 技术栈对比

| 维度 | Hermes Agent | OpenClaw |
|------|-------------|----------|
| **语言** | Python 3.11+ | TypeScript (Node 22+) |
| **包管理** | uv, pip | pnpm |
| **构建** | 无（Python 直接运行） | tsdown, tsc, oxfmt |
| **测试** | pytest (17k tests, 900 files) | vitest (colocated *.test.ts) |
| **CLI** | Rich, prompt_toolkit | Commander (Node CLI) |
| **UI** | Ink (React) TUI | React Control UI |
| **数据库** | SQLite (FTS5) | 文件系统存储 |
| **Gateway** | WebSocket, HTTP | WebSocket, HTTP |
| **消息平台** | 20+ 适配器 | 25+ 通道 |
| **配置** | YAML (config.yaml) + .env | JSON (openclaw.json) |
| **技能格式** | SKILL.md (YAML frontmatter) | SKILL.md (YAML frontmatter) |
| **记忆** | 多提供者（Honcho, Mem0 等） | 文件优先，外部提供者 |

## 16. 测试策略

### Hermes Agent

**测试套件**: `tests/` (~17k tests, ~900 files)

**关键设计**:
- 使用 `scripts/run_tests.sh` 运行（不直接调用 pytest）
- 强制 hermetic 环境（取消凭证变量，TZ=UTC, LANG=C.UTF-8, 4 xdist workers）
- Profile 测试模拟 `Path.home()` 和设置 `HERMES_HOME`
- 测试不写入 `~/.hermes/`（使用临时目录）
- 技能测试位于 `tests/skills/test_<skill>_skill.py`

**测试原则**:
- 不写 change-detector 测试
- 测试行为和 invariant，而非特定数据
- CI 等价本地运行

### OpenClaw

**测试套件**: Vitest，colocated `*.test.ts`，e2e `*.e2e.test.ts`

**关键设计**:
- `pnpm test <path-or-filter>` — 不直接运行 vitest
- 测试 worker 最大 16。内存压力：`OPENCLAW_VITEST_MAX_WORKERS=1`
- 实时测试：`OPENCLAW_LIVE_TEST=1`
- 扩展测试：`pnpm test:extensions`
- 类型检查：`tsgo` lanes 只有（不添加 `tsc --noEmit`）

**测试原则**:
- 偏好行为测试而非 workflow/docs string greps
- 注入和窄 mock 优于 broad barrels
- 干净计时器/环境/globals/mocks/sockets/module 状态

## 17. 安全模型对比

### Hermes Agent

**安全特性**:
- **DM 配对**: 未知发送者接收配对码，bot 不处理其消息
- **命令审批**: 工具调用可以需要审批
- **沙箱**: Docker, SSH, Singularity 等终端后端提供隔离
- **插件隔离**: 插件不得修改核心文件
- **Profile 隔离**: 每个 profile 有自己的配置和密钥

### OpenClaw

**安全特性**:
- **DM 配对策略**: 默认 `dmPolicy="pairing"`，公开 inbound 需要显式选择加入
- **沙箱模式**: `agents.defaults.sandbox.mode: "non-main"` 运行非主会话在沙箱中
- **工具策略**: 基于策略的工具可见性和执行
- **Owner 授权**: 工具和通道操作需要 owner 授权
- **配置合同**: 退役的公共键保持退役，兼容在原始迁移/doctor 中

## 18. 总结与建议

### 核心架构差异总结

| 维度 | Hermes Agent | OpenClaw |
|------|-------------|----------|
| **架构风格** | 单体核心（16k LOC AIAgent 类） | Monorepo 模块化（严格插件边界） |
| **语言** | Python | TypeScript |
| **代理循环** | 同步，预算控制 | 流式，嵌入式 |
| **插件系统** | 多层，钩子驱动 | 严格 SDK 边界，惰性加载 |
| **工具系统** | 注册表 + 工具集 | 核心 + 插件工具 |
| **记忆系统** | 多提供者，ABC 合约 | 文件优先，简单模型 |
| **会话存储** | SQLite (FTS5) | 文件系统 |
| **Gateway** | 平台适配器 | WebSocket/HTTP 协议 |
| **技能加载** | 目录扫描，frontmatter | Clawhub 市场，workspace 技能 |
| **委派系统** | delegate_task 工具 | 子代理注册表 |
| **测试策略** | pytest, hermetic | vitest, 注入优先 |

### 设计思路对比

**Hermes Agent** 的设计思路是**自进化代理**：
- 强调学习循环和知识持久性
- 代理从经验中学习，创建和改进技能
- 支持研究用途（RL 训练、轨迹生成）
- 灵活性优先，支持多种模型、平台、工具
- 单体核心但通过插件系统扩展

**OpenClaw** 的设计思路是**个人 AI 助手**：
- 强调本地优先和性能
- 严格的插件边界和核心隔离
- 惰性加载和热路径优化
- 多通道统一体验
- 原生应用伴侣

### 适用场景建议

**选择 Hermes Agent 如果**:
- 需要自进化、学习能力的代理
- 需要研究功能（RL 训练、轨迹压缩）
- 偏好 Python 生态
- 需要灵活的工具和模型支持
- 需要多 profile 隔离

**选择 OpenClaw 如果**:
- 需要高性能、本地优先的个人助手
- 需要严格插件边界和核心隔离
- 偏好 TypeScript 生态
- 需要原生应用（macOS, iOS, Android）
- 需要多通道统一体验

### 架构设计建议

1. **单体 vs 模块化**: 单体核心（Hermes）更容易理解和调试，但模块化（OpenClaw）提供更好的扩展性和维护性
2. **同步 vs 流式**: 流式（OpenClaw）提供更好用户体验，但同步（Hermes）更容易实现和控制
3. **钩子 vs SDK 边界**: 钩子系统（Hermes）更灵活，但 SDK 边界（OpenClaw）提供更好的隔离和稳定性
4. **数据库 vs 文件存储**: SQLite（Hermes）提供更好的查询能力，文件系统（OpenClaw）更简单和透明
5. **注册表 vs 惰性加载**: 注册表（Hermes）更简单直接，惰性加载（OpenClaw）提供更好的启动性能