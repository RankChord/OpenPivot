# 一人公司 Agent 框架 — 架构设计规格

> 版本: v1.0 | 日期: 2026-05-15
> 状态: 已批准

---

## 1. 项目概述

### 1.1 目标定位

个人/一人公司级 **AI Agent 框架**，用于：
- **项目设计与研发** — 辅助代码设计、开发、测试、部署全生命周期
- **行政办公与管理** — 文档处理、邮件处理、日程安排、报告生成

### 1.2 设计目标

| 优先级 | 目标 | 说明 |
|--------|------|------|
| P0 | 代码开发效率 | 能稳定完成文件读写、代码编写、测试运行、Git 操作 |
| P0 | 系统稳定性 | Worker 崩溃不影响核心，自动重启 |
| P1 | API 成本控制 | Prompt 缓存命中最大化，避免 token 浪费 |
| P1 | 权限安全 | 精细到工具级别的文件路径/目录限制 |
| P1 | 可扩展性 | 新功能通过 plugins 热插拔 |
| P2 | 多平台 | macOS / Linux 原生，Windows 可运行 |

### 1.3 参考来源与设计取舍

| 参考项目 | 核心采纳 | 核心舍弃 | 原因 |
|----------|---------|---------|------|
| **Claude Code** | 完整工具接口、任务状态机、权限引擎、Feature Flags | React UI、云端依赖 | 我们是独立的本地框架，不需要云端 |
| **Hermes Agent** | Hook 系统、工具集、Curator、多提供者记忆 | Python 单体核心(16k LOC) | Rust 要求模块化和安全性 |
| **OpenClaw** | SDK 边界隔离、惰性加载、崩溃恢复 | Monorepo 复杂度 | dylib 隔离更彻底 |
| **Superpowers** | SKILL.md 标准、技能生命周期 | 仅作为技能格式规范 | 其他能力由框架原生实现 |

### 1.4 术语表

| 术语 | 定义 |
|------|------|
| Core | 核心进程，管理代理循环、会话、Prompt |
| Worker | 进程隔离的可执行体，加载 dylib 工具 |
| Tool | 工具，Agent 可调用的具体能力 |
| Hook | 生命周期回调，插件可注册 |
| Skill | 技能，SKILL.md 描述的指令集 |
| Provider | 提供者，如 LLM Provider、Memory Provider |

---

## 2. 核心架构

### 2.1 系统整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Core 进程                               │
│                                                             │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌────────────┐  │
│  │ AgentLoop │ │TaskManager│ │SessionDB  │ │ToolRegistry│  │
│  └──────┬────┘ └──────┬────┘ └──────┬────┘ └──────┬─────┘  │
│         │             │             │             │         │
│  ┌──────▼────┐ ┌──────▼────┐ ┌──────▼────┐ ┌──────▼─────┐  │
│  │LLMClient  │ │Permission │ │SkillEngine│ │PluginMgr   │  │
│  │(多提供商) │ │Engine     │ │(SKILL.md) │ │(dylib)     │  │
│  └──────┬────┘ └──────┬────┘ └──────┬────┘ └──────┬─────┘  │
│         │             │             │             │         │
│  ┌──────▼────┐ ┌──────▼────┐ ┌──────▼────┐ ┌──────▼─────┐  │
│  │PromptCache│ │MemoryMgr  │ │IPC Gateway│ │FeatureFlags│  │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘  │
└────┬────────────────────────────────────────────┬──────────┘
     │ Unix Socket                                 │ Unix Socket
     ▼                                             ▼
┌──────────────┐                          ┌──────────────┐
│ Worker A     │                          │ Worker B     │
│ Shell+Git+   │                          │ File+Web+    │
│ Code Tools   │                          │ Search Tools │
└──────────────┘                          └──────────────┘
```

### 2.2 目录结构

```
agent-framework/
├── Cargo.toml                  # workspace 根
├── crates/
│   ├── agent-core/            # 微内核（Agent Loop）
│   ├── agent-tools/           # 工具注册与接口定义
│   ├── agent-plugin-sdk/      # 插件 SDK（纯 trait，零依赖）
│   ├── agent-gateway/         # IPC Gateway
│   ├── agent-worker/          # Worker 进程运行器
│   ├── agent-skills/          # SKILL.md 解析与引擎
│   ├── agent-memory/          # 记忆系统（多提供者）
│   ├── agent-cron/            # 定时任务
│   ├── agent-delegate/        # 委托/子代理
│   ├── agent-permission/      # 权限引擎
│   ├── agent-llm/             # LLM 客户端（多提供商）
│   ├── agent-prompt/          # Prompt 构建与缓存
│   ├── agent-tui/             # TUI（ratatui）
│   └── agent-config/          # 配置管理（TOML）
├── tools/                     # 内置工具 dylib
│   ├── shell/
│   ├── file/
│   ├── git/
│   ├── browser/
│   ├── lsp/
│   └── ...
├── extensions/                # 社区插件
├── skills/                    # 内置技能包
├── config/                    # 配置模板
└── bin/
    ├── agent-core.rs          # Core 进程入口
    ├── agent-gateway.rs       # Gateway 进程入口
    └── agent-tui.rs           # TUI 入口
```

### 2.3 进程通信协议

**JSON-RPC 2.0 over Unix Socket**:

```json
{"jsonrpc": "2.0", "id": 1, "method": "tool.call", "params": {
  "name": "read_file",
  "path": "src/main.rs",
  "tool_id": "tool_abc123"
}}
{"jsonrpc": "2.0", "id": 1, "result": {"ok": true, "content": "..."}}
```

请求/应答结构：
- Core → Worker：`tool.call`、`tool.cancel`、`health.check`
- Worker → Core：`tool.progress`、`tool.result`、`health.status`
- 多路复用：每个请求带 `tool_id`，Core 通过该 ID 路由响应

---

## 3. 代理循环设计

### 3.1 Agent Loop 主流程

```rust
// agent-core/src/loop.rs
pub struct AgentLoop {
    pub session_id: String,
    pub budget: IterationBudget,
    pub max_iterations: u32,
    // ...
}

impl AgentLoop {
    pub async fn run(&self, user_message: &str) -> Result<AgentOutput> {
        let mut messages = self.build_context(user_message).await?;
        self.prefill_system_prompt(&mut messages).await?;

        let mut iteration = 0u32;
        while iteration < self.max_iterations && !self.budget.exhausted() {
            // 1. 构建 API 消息
            let api_messages = self.api_messages(&mut messages)?;

            // 2. 调用 LLM（支持流式）
            let response = self.llm_client.chat(api_messages).await?;

            // 3. 处理响应
            if response.has_tool_calls() {
                // 3a. 工具调用：路由到 Tool Registry
                for tool_call in response.tool_calls() {
                    let result = self.tool_registry.execute(tool_call).await?;
                    messages.append_tool_result(tool_call, result);
                }
                iteration += 1;
            } else {
                // 3b. 文本响应：返回最终答案
                return Ok(AgentOutput::Final(response.text()));
            }
        }

        // 预算耗尽 → 生成总结
        Ok(AgentOutput::BudgetExhausted(self.summarize().await?))
    }
}
```

### 3.2 预算控制

```rust
pub struct IterationBudget {
    pub max_iterations: u32,          // 默认 90
    pub max_tokens: Option<u64>,      // 可选 token 上限
    pub grace_call: bool,             // 耗尽时允许最后一回合总结
}
```

### 3.3 流式传输设计

Core 支持两种模式：
1. **同步模式**（Hermes 模式）：等待完整响应，适合批量任务和低 API 成本
2. **流式模式**（Claude Code 模式）：实时 token 传输，TUI 显示进度

模式切换由 `config.yaml` 的 `agent.stream_mode: "full" | "token"` 控制。

### 3.4 Prompt 缓存策略

**Claude Code 的关键洞察**：Prompt 缓存命中率直接影响 API 成本。

```
System Prompt (固定，5000 tokens)
├── Identity + Persona
├── Skills (已加载的 SKILL.md)
├── Tool Definitions (动态但缓存)
├── Memory Context
└── Context Files (AGENTS.md 等)

Conversation History (可变)
├── User Messages
├── Assistant Messages
└── Tool Results
```

缓存策略：
- System Prompt 在会话内完全不变 → Anthropic `cache_control` 或 OpenAI `prefix`
- Tool Definitions 变化时自动失效（Registry generation counter）
- 中间对话轮次通过辅助模型摘要压缩

---

## 4. 工具系统

### 4.1 Tool Trait（结合 Claude Code Tool.ts 完整接口）

```rust
// agent-tools/src/tool.rs
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: JsonSchema,       // JSON Schema v2020-12
    pub output_schema: Option<JsonSchema>,
    pub toolset_name: String,          // Hermes 风格分组
    pub aliases: Vec<String>,          // 向后兼容
    pub max_result_size: usize,        // 结果最大字符，超出落盘
}

#[async_trait]
pub trait Tool: Send + Sync + std::fmt::Debug {
    fn definition(&self) -> &ToolDefinition;
    
    /// 核心执行方法
    async fn call(
        &self,
        input: serde_json::Value,
        ctx: &ToolUseContext<'_>,
        can_use: &dyn Fn(&PermissionCheck) -> PermissionResult,
        progress: &dyn Fn(ProgressUpdate),
    ) -> ToolResult;
    
    /// Claude Code: 是否线程安全（可并行）
    fn is_concurrency_safe(&self) -> bool { true }
    
    /// Claude Code: 是否只读
    fn is_read_only(&self) -> bool { false }
    
    /// Claude Code: 是否破坏性操作
    fn is_destructive(&self) -> bool { false }
    
    /// Claude Code: 中断策略
    fn interrupt_behavior(&self) -> InterruptPolicy { InterruptPolicy::Block }
    
    /// Claude Code: 权限检查（工具级别）
    async fn check_permissions(
        &self,
        input: &serde_json::Value,
        ctx: &PermissionContext,
    ) -> PermissionResult;
    
    /// Hermes: 环境/依赖检查
    fn requirements_check(&self) -> bool { true }
    
    /// Claude Code: 动态 Prompt 描述
    async fn prompt_description(&self, ctx: &ToolPromptContext<'_>) -> String;
}

pub enum InterruptPolicy {
    Cancel,     // 中断时取消
    Block,      // 中断时阻塞
}
```

### 4.2 ToolUseContext（参考 Claude Code's ToolUseContext）

```rust
pub struct ToolUseContext<'a> {
    pub session_id: &'a str,
    pub agent_id: Option<&'a str>,
    pub tool_use_id: &'a str,
    pub permission_context: &'a PermissionContext,
    pub workspace_dir: &'a Path,
    pub messages: &'a [Message],
    
    // 回调：修改会话状态
    pub set_state: &'a dyn Fn(SessionStateUpdate),
    
    // 回调：追加系统消息
    pub append_system_message: &'a dyn Fn(SystemMessage),
    
    // 回调：请求用户交互
    pub request_prompt: Option<&'a dyn Fn(PromptRequest) -> PromptResponse>,
}
```

### 4.3 Tool Registry

```rust
// agent-tools/src/registry.rs
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Box<dyn Tool>>>,
    toolsets: RwLock<HashMap<String, ToolSet>>,
    generation: AtomicU64,           // 每次注册/注销递增
    description_cache: Arc<RwLock<Option<ToolDescriptionCache>>>,
}

impl ToolRegistry {
    /// 注册工具（插件调用）
    pub fn register(&self, tool: Box<dyn Tool>) {
        self.tools.write().insert(tool.name().into(), tool);
        self.generation.fetch_add(1, Ordering::Relaxed);
        self.description_cache.write().take(); // 使缓存失效
    }
    
    /// 获取 OpenAI 格式工具定义（带缓存）
    pub fn get_definitions(&self) -> Vec<ToolSchema> {
        let gen = self.generation.load(Ordering::Relaxed);
        if let Some(cached) = &*self.description_cache.read() {
            if cached.generation == gen { return cached.definitions.clone(); }
        }
        let definitions = self._build_definitions();
        *self.description_cache.write() = Some(ToolDescriptionCache {
            generation: gen,
            definitions: definitions.clone(),
        });
        definitions
    }
    
    /// 执行工具调用
    pub async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let tool = self.tools.read().get(&call.name)
            .ok_or_else(|| Error::ToolNotFound(call.name.clone()))?;
        
        // 权限检查
        let perm_result = tool.check_permissions(&call.input, call.ctx.perm()).await?;
        match perm_result.behavior {
            PermissionBehavior::Allow => {},
            PermissionBehavior::Ask => { /* 等待用户确认 */ },
            PermissionBehavior::Deny => return Err(Error::PermissionDenied),
        }
        
        // 执行
        tool.call(call.input, call.ctx, can_use_fn, progress_fn).await
    }
}
```

### 4.4 工具执行并发策略

```rust
// 并发执行
async fn execute_parallel(&self, tool_calls: Vec<ToolCall>) -> Vec<ToolResult> {
    let (safe, unsafe): (Vec<_>, Vec<_>) = tool_calls.into_iter()
        .partition(|tc| self.registry.get(&tc.name).is_concurrency_safe());
    
    // 安全工具并行，不安全工具串行
    let safe_results = futures::future::join_all(
        safe.into_iter().map(|tc| self.execute_tool(tc))
    ).await;
    
    let mut unsafe_results = Vec::new();
    for tc in unsafe {
        unsafe_results.push(self.execute_tool(tc).await);
    }
    // 合并结果
    merge_results(safe_results, unsafe_results)
}
```

### 4.5 默认值与构建器

```rust
// agent-tools/src/builder.rs
pub struct BuildTool<D> {
    pub definition: ToolDefinition,
    pub call: D,
    // 可选方法有默认值
    pub concurrency_safe: bool,
    pub read_only: bool,
    pub destructive: bool,
    pub interrupt_behavior: InterruptPolicy,
}

impl BuildTool {
    pub fn new(name: &str, schema: JsonSchema) -> Self { /* ... */ }
    
    // 流畅 API
    pub fn concurrency_safe(mut self, val: bool) -> Self { self.concurrency_safe = val; self }
    pub fn read_only(mut self, val: bool) -> Self { self.read_only = val; self }
    pub fn destructive(mut self, val: bool) -> Self { self.destructive = val; self }
    
    pub fn build(self) -> Box<dyn Tool> {
        Box::new(GeneratedTool { /* ... */ })
    }
}
```

---

## 5. 插件/扩展系统

### 5.1 插件架构

```
┌─────────────────────────────────────┐
│           Core 进程                  │
│  ┌──────────────┐                   │
│  │ PluginMgr    │                   │
│  │ - 元数据管理  │                   │
│  │ - 生命周期    │                   │
│  │ - Worker 池  │                   │
│  └──────┬───────┘                   │
│         │ fork() + exec()           │
│         ▼                           │
│  ┌──────────────┐  ┌──────────────┐ │
│  │ Worker A     │  │ Worker B     │ │
│  │ libshell.so  │  │ libgit.so    │ │
│  │ libcode.so   │  │ libweb.so    │ │
│  │              │  │              │ │
│  │ Plugin API:  │  │ Plugin API:  │ │
│  │ - register  │  │ - register  │ │
│  │ - init      │  │ - init      │ │
│  │ - shutdown  │  │ - shutdown  │ │
│  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────┘
```

### 5.2 插件加载器

```rust
// agent-plugin-sdk/src/loader.rs
use libloading::{Library, Symbol};

pub struct PluginHandle {
    library: Library,
    worker_pid: u32,
    plugin_id: String,
}

impl PluginHandle {
    /// 加载 dylib 到独立 Worker 进程
    pub fn load(plugin_dir: &Path, plugin_id: &str) -> Result<Self> {
        let lib_path = plugin_dir.join(format!("lib{}.so", plugin_id));
        
        // 1. 启动 Worker 进程
        let worker = WorkerProcess::spawn()?;
        
        // 2. Worker 进程加载 dylib
        let library = unsafe { Library::new(lib_path)? };
        
        // 3. 获取插件注册函数
        let register_fn: Symbol<fn() -> PluginRegistration> = 
            unsafe { library.get(b"register_plugin")? };
        let registration = register_fn();
        
        // 4. Worker 进程向 Core 注册工具
        worker.register_tools(registration.tools)?;
        
        Ok(PluginHandle {
            library,
            worker_pid: worker.pid(),
            plugin_id: plugin_id.into(),
        })
    }
    
    /// 热替换：卸载旧版本，加载新版本
    pub fn hot_reload(&self, new_version: &Path) -> Result<Self> {
        // 1. 标记旧插件停止
        // 2. 等待进行中的请求完成
        // 3. 加载新版本
        // 4. 注册新工具
    }
}
```

### 5.3 插件注册函数（C ABI）

```rust
// tools/shell/src/lib.rs （插件侧）
use agent_plugin_sdk::{PluginRegistration, ToolBuilder};

#[no_mangle]
pub extern "C" fn register_plugin() -> PluginRegistration {
    PluginRegistration {
        id: "shell".into(),
        version: "0.1.0".parse().unwrap(),
        tools: vec![
            ToolBuilder::new("execute_command", execute_schema())
                .call(execute_command_fn)
                .concurrency_safe(false)
                .destructive(true)
                .build(),
            ToolBuilder::new("read_terminal_output", read_output_schema())
                .call(read_terminal_output_fn)
                .concurrency_safe(true)
                .read_only(true)
                .build(),
        ],
        hooks: vec![
            ("pre_tool_call", Box::new(shell_pre_tool_call)),
        ],
    }
}
```

### 5.4 Hook 系统（Hermes 风格）

```rust
// agent-plugin-sdk/src/hooks.rs
pub type HookFn = Box<dyn Fn(&HookContext<'_>) -> Result<()> + Send + Sync>;

pub enum HookPoint {
    PreToolCall,
    PostToolCall,
    PreLlmCall,
    PostLlmCall,
    OnSessionStart,
    OnSessionEnd,
    OnSessionReset,
    SubagentStop,
    PreApprovalRequest,
    PostApprovalResponse,
}

pub struct HookRegistry {
    hooks: HashMap<HookPoint, Vec<(String, HookFn)>>,
}

impl HookRegistry {
    pub fn register(&mut self, plugin_id: &str, point: HookPoint, func: HookFn) {
        self.hooks.entry(point).or_insert_with(Vec::new)
            .push((plugin_id.into(), func));
    }
    
    pub fn execute(&self, point: HookPoint, ctx: &HookContext<'_>) -> Result<()> {
        if let Some(handlers) = self.hooks.get(&point) {
            for (id, func) in handlers {
                func(ctx).map_err(|e| Error::HookError(id.clone(), e))?;
            }
        }
        Ok(())
    }
}
```

### 5.5 Worker 进程管理器

```rust
// agent-worker/src/process.rs
pub struct WorkerPool {
    workers: HashMap<String, WorkerProcess>,
    health_interval: Duration,
    max_restarts: u32,
}

impl WorkerPool {
    pub fn spawn_worker(&mut self, plugin_id: &str, lib_path: &Path) -> Result<()> {
        let worker = WorkerProcess::new(plugin_id, lib_path)?;
        worker.start()?;
        self.workers.insert(plugin_id.into(), worker);
        Ok(())
    }
    
    /// 健康检查循环
    pub async fn health_loop(&mut self) {
        let mut interval = tokio::time::interval(self.health_interval);
        loop {
            interval.tick().await;
            self.check_workers().await;
        }
    }
    
    /// 崩溃自动重启
    async fn check_workers(&mut self) {
        let dead_workers: Vec<_> = self.workers.iter()
            .filter(|(_, w)| !w.is_alive())
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in dead_workers {
            if self.restart_count(&id) < self.max_restarts {
                self.restart_worker(&id).await;
            } else {
                log::error!("Worker {} exceeded max restarts, disabling", id);
            }
        }
    }
}
```

---

## 6. 技能系统

### 6.1 SKILL.md 格式（Superpowers 标准）

```yaml
---
name: Git Workflow
description: 管理 Git 工作流，包括分支、合并、冲突解决。
version: 1.0.0
author: Your Name
platforms: [linux, macos]
metadata:
  tags: [dev, version-control]
  category: developer
  related_skills: [code-review, github]
  required_tools: [execute_command, read_file, write_file]
---

# Git Workflow Skill

## When to Use
When the user asks to perform any git operation...

## Procedure
1. Check current git status
2. Perform the requested operation
3. Verify the result
```

### 6.2 Skill Engine

```rust
// agent-skills/src/engine.rs
pub struct SkillEngine {
    skills: HashMap<String, SkillDefinition>,
    usage_tracker: SkillUsageTracker,
    curator: Curator,
    config_vars: HashMap<String, String>,
}

impl SkillEngine {
    /// 扫描所有技能
    pub fn scan_skills(&mut self, dirs: &[PathBuf]) -> Result<()> {
        for dir in dirs {
            for entry in walkdir::WalkDir::new(dir) {
                if entry.file_name() == "SKILL.md" {
                    let skill = self.parse_skill_file(entry.path())?;
                    if self.should_load(&skill) {
                        self.skills.insert(skill.name.clone(), skill);
                    }
                }
            }
        }
        Ok(())
    }
    
    /// 检查是否加载（YAGNI：只加载需要的）
    fn should_load(&self, skill: &SkillDefinition) -> bool {
        // 1. 平台兼容
        if !skill.platforms.contains(&current_platform()) { return false; }
        // 2. 工具可用
        if !self.has_required_tools(&skill.metadata.required_tools) { return false; }
        // 3. 未被禁用
        if self.is_disabled(&skill.name) { return false; }
        true
    }
    
    /// 构建技能 Prompt
    pub fn build_prompt(&self, active_skills: &[String]) -> String {
        let mut prompt = String::from("# Active Skills\n\n");
        for name in active_skills {
            if let Some(skill) = self.skills.get(name) {
                prompt.push_str(&skill.system_prompt());
            }
        }
        prompt
    }
}
```

### 6.3 Curator（Hermes 自动归档）

```rust
// agent-skills/src/curator.rs
pub struct Curator {
    usage_log: SkillUsageLog,
    archive_dir: PathBuf,
    stale_after_days: u32,
    archive_after_days: u32,
}

impl Curator {
    /// 定期运行（默认每天）
    pub async fn run(&self) -> Result<()> {
        let usage = self.usage_log.load()?;
        for (skill, stats) in usage.iter() {
            if self.should_archive(skill, stats)? {
                self.archive_skill(skill)?;
            } else if self.should_stale(skill, stats)? {
                self.mark_stale(skill)?;
            }
        }
        Ok(())
    }
    
    fn should_archive(&self, skill: &str, stats: &UsageStats) -> bool {
        // Pinned 技能永不归档
        if self.is_pinned(skill) { return false; }
        // 超过归档时间
        stats.last_used.days_ago() > self.archive_after_days
        // 只有 Agent 创建的技能才归档
        && stats.created_by == "agent"
    }
}
```

---

## 7. 记忆系统

### 7.1 Memory Provider Trait

```rust
// agent-memory/src/provider.rs
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    fn name(&self) -> &str;
    
    /// 初始化（连接、预热）
    async fn initialize(&mut self, session_id: &str) -> Result<()>;
    
    /// 预取（下一轮上下文召回）
    async fn prefetch(&self, query: &str) -> Result<String>;
    
    /// 异步召回（为下一轮做准备）
    fn queue_prefetch(&self, query: String);
    
    /// 同步完成的回合
    async fn sync_turn(&self, user: &str, assistant: &str) -> Result<()>;
    
    /// 系统提示上下文
    async fn system_prompt_block(&self) -> Result<String>;
    
    /// 关闭
    async fn shutdown(&mut self) -> Result<()>;
}
```

### 7.2 Memory Manager

```rust
// agent-memory/src/manager.rs
pub struct MemoryManager {
    builtin: BuiltinMemory,                // 内置文件记忆
    provider: Option<Box<dyn MemoryProvider>>, // 外部提供者（最多一个）
    active_session: String,
    config: MemoryConfig,
}

impl MemoryManager {
    /// 构建记忆上下文块
    pub async fn build_context_block(&mut self) -> Result<String> {
        let mut block = String::new();
        
        // 1. 内置记忆（MEMORY.md, USER.md, etc）
        block.push_str(&self.builtin.context_block()?);
        
        // 2. 外部提供者
        if let Some(provider) = &self.provider {
            let query = self.build_query()?;
            let external = provider.prefetch(&query).await?;
            if !external.is_empty() {
                block.push_str(&format!("<memory-context>\n{}\n</memory-context>\n", external));
            }
        }
        Ok(block)
    }
}
```

### 7.3 内置记忆文件

```
~/.agent/workspace/
├── MEMORY.md          # 跨会话持久化记忆
├── USER.md            # 用户画像
├── SOUL.md            # 代理人格
├── AGENTS.md          # 项目上下文（Claude Code 风格）
└── TOOLS.md           # 工具使用指南
```

---

## 8. Gateway 架构

### 8.1 IPC Gateway

```
Core ↔ Gateway ↔ Worker A
              ↔ Worker B
              ↔ Worker C
```

Gateway 职责：
- **多路复用**：同时处理多个 Worker 的请求
- **路由**：按工具名路由到正确的 Worker
- **负载均衡**：相同工具的不同实现之间
- **背压控制**：防止 Worker 被请求淹没

### 8.2 JSON-RPC 方法

| 方法 | 方向 | 说明 |
|------|------|------|
| `tool.call` | Core → Worker | 执行工具调用 |
| `tool.cancel` | Core → Worker | 取消进行中的工具 |
| `tool.progress` | Worker → Core | 流式进度更新 |
| `tool.result` | Worker → Core | 工具执行结果 |
| `health.check` | Core → Worker | 健康检查 |
| `health.status` | Worker → Core | 心跳响应 |
| `register.tools` | Worker → Core | 注册工具定义 |

### 8.3 Unix Socket 多路复用

```rust
// agent-gateway/src/multiplexer.rs
pub struct Multiplexer {
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<ToolResult>>>>,
}

impl Multiplexer {
    pub async fn call_tool(&self, worker: &UnixStream, call: ToolCall) -> Result<ToolResult> {
        let (tx, rx) = oneshot::channel();
        let id = uuid::random().to_string();
        
        self.pending.lock().insert(id.clone(), tx);
        worker.write_json(&ToolRpcRequest::new(&id, "tool.call", &call)).await?;
        
        rx.await.map_err(|_| Error::WorkerDisconnected)?
    }
    
    pub async fn handle_response(&self, message: ToolRpcResponse) -> Result<()> {
        if let Some(tx) = self.pending.lock().remove(&message.id) {
            let _ = tx.send(ToolResult { ok: message.result });
        }
        Ok(())
    }
}
```

---

## 9. 通道/平台适配器

### 9.1 适配器架构

```rust
// agent-platforms/src/adapter.rs
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn name(&self) -> &str;
    
    async fn connect(&self) -> Result<()>;
    async fn disconnect(&self) -> Result<()>;
    
    /// 发送消息
    async fn send(&self, chat_id: &str, message: &Message) -> Result<SendResult>;
    
    /// 编辑消息（支持流式草稿的平台）
    async fn edit_message(&self, chat_id: &str, message_id: &str, content: &str) -> Result<()>;
    
    /// 删除消息
    async fn delete_message(&self, chat_id: &str, message_id: &str) -> Result<()>;
    
    /// 可选：打字指示器
    fn show_typing_indicator(&self, _chat_id: &str) {}
}
```

### 9.2 平台注册表

```
内置平台（自动加载）:
├── cli/              # 命令行界面
├── tui/              # 终端 UI
└── webchat/          # Web 聊天

用户安装（选择加入）:
├── telegram/
├── discord/
├── slack/
└── weixin/           # 微信
```

---

## 10. 会话管理

### 10.1 会话键格式

```
agent:{agent_id}:{platform}:{chat_type}:{chat_id}[:{extra}]

示例:
agent:main:cli:direct:user_terminal:
agent:main:telegram:private:123456789:
```

### 10.2 SessionDB（SQLite）

```rust
// agent-core/src/session.rs
pub struct SessionDB {
    pool: SqlitePool,
}

impl SessionDB {
    pub fn new(db_path: &Path) -> Result<Self> {
        let pool = SqlitePool::connect(db_path).await?;
        Self::create_tables(&pool).await?;
        Ok(SessionDB { pool })
    }
    
    /// 保存消息
    pub async fn save_message(&self, msg: &Message) -> Result<()> {
        sqlx::query!(
            "INSERT INTO messages (session_id, role, content, timestamp, tool_calls)
             VALUES ($1, $2, $3, $4, $5)",
            msg.session_id, msg.role, msg.content, msg.timestamp, msg.tool_calls_json
        ).execute(&self.pool).await?;
        Ok(())
    }
    
    /// FTS5 全文搜索
    pub async fn search_sessions(&self, query: &str) -> Result<Vec<SessionSummary>> {
        // 使用 SQLite FTS5 扩展
    }
}
```

### 10.3 会话压缩

```
原始对话:
┌─────────────────────────────┐
│ System Prompt (缓存固定)    │
├─────────────────────────────┤
│ 用户: 帮我看看这个文件      │ ← 保留（头部）
│ AI:   文件内容是...         │ ← 保留（头部）
├─────────────────────────────┤
│ 用户: 改一下第三行          │
│ AI:   已修改                │ ← 摘要压缩（中间）
│ 用户: 再改一下第五行        │
│ AI:   已修改                │ ←
│ 用户: 运行测试              │
│ AI:   测试通过              │ ←
├─────────────────────────────┤
│ 用户: 提交并推送到远端      │ ← 保留（尾部）
│ AI:   已完成                │ ←
└─────────────────────────────┘
```

---

## 11. 委派/子代理系统

### 11.1 委派模式

```rust
// agent-delegate/src/executor.rs
pub enum DelegateMode {
    Single,        // 单个子代理
    Batch {        // 批量并发
        tasks: Vec<Task>,
        max_concurrent: u32,    // 默认 3
    },
}

pub struct DelegateTask {
    pub goal: String,
    pub context: Option<String>,
    pub toolsets: Option<Vec<String>>,
    pub role: DelegateRole,
}

pub enum DelegateRole {
    Leaf,             // 专注 worker，无法再委派
    Orchestrator,     // 可继续委派（深度限制）
}
```

### 11.2 子代理生命周期

```
spawn → initialize → execute → summarize → cleanup
  │                                          │
  ├─ 创建独立 SessionDB entry                 ├─ 清理临时目录
  ├─ 克隆工具权限上下文                       ├─ 保存输出
  ├─ 设置 budget 和 timeout                   └─ 触发父代理继续
  └─ 启动独立进程/线程
```

---

## 12. 定时任务系统

### 12.1 Cron 架构

```rust
// agent-cron/src/scheduler.rs
pub struct CronScheduler {
    jobs: CronJobStore,
    active_jobs: HashMap<String, ActiveJob>,
    interval: Duration,
}

impl CronScheduler {
    /// Tick 循环
    pub async fn tick_loop(&mut self) {
        let mut interval = tokio::time::interval(self.interval);
        loop {
            interval.tick().await;
            self.check_jobs().await;
        }
    }
    
    async fn check_jobs(&mut self) {
        let due = self.jobs.get_due_jobs();
        for job in due {
            if self.is_running(&job.id) { continue; }
            self.execute_job(job).await;
        }
    }
    
    /// 3 分钟硬中断
    async fn execute_job(&mut self, job: CronJob) {
        let timeout = tokio::time::timeout(
            Duration::from_secs(180),
            self.spawn_agent_for_job(job),
        );
        match timeout.await {
            Ok(result) => self.record_result(result),
            Err(_) => self.record_timeout(),
        }
    }
}
```

### 12.2 支持格式

| 格式 | 示例 | 说明 |
|------|------|------|
| 持续时间 | `30m`, `2h`, `1d` | 延迟执行 |
| Every 短语 | `every 2h`, `every monday 9am` | 周期性重复 |
| Cron 表达式 | `0 9 * * *` | 5 字段标准 cron |
| ISO 时间戳 | `2026-06-01T09:00:00Z` | 一次性 |

---

## 13. 关键设计决策

### 13.1 系统提示不可变性

**决策**：Session 内系统提示只构建一次，绝不改变。

**理由**：
- Anthropic prefix caching 要求字节完全一致
- 改变系统提示 → 整个缓存失效 → 成本飙升 3-5x

**实现**：
```rust
// 启动时构建一次
let system_prompt = build_system_prompt(session).await?;
let _cache_handle = self.llm_client.cache_prefix(&system_prompt).await?;

// 后续轮次复用
loop {
    let messages = self.api_messages(&history)?;
    messages.insert(0, system_prompt.clone()); // 相同字节
    let response = self.llm_client.chat(messages).await?;
}
```

### 13.2 工具定义记忆化

**决策**：工具定义带 generation counter 做缓存失效。

**实现**：
```rust
pub struct ToolDescriptionCache {
    pub generation: u64,
    pub definitions: Vec<ToolSchema>,
}

impl ToolRegistry {
    pub fn get_definitions(&self) -> Vec<ToolSchema> {
        let gen = self.generation.load(Relaxed);
        if let Some(cached) = &*self.cache.read() {
            if cached.generation == gen { return cached.definitions.clone(); }
        }
        // 重建缓存
    }
}
```

### 13.3 安全工具执行

**决策**：工具永远不崩溃 Agent 循环。所有错误分类返回。

```rust
async fn execute_tool(&self, call: ToolCall) -> ToolResult {
    match self.registry.call(call).await {
        Ok(result) => ToolResult::Success(result),
        Err(Error::PermissionDenied) => ToolResult::PermissionDenied,
        Err(Error::ToolNotFound(name)) => ToolResult::ToolNotFound(name),
        Err(Error::Timeout) => ToolResult::Timeout,
        Err(e) => {
            let classification = classify_error(&e);
            ToolResult::Error(classification)
        }
    }
}
```

### 13.4 崩溃隔离

**决策**：Worker 崩溃不影响 Core，Core 自动重启 Worker。

```
Core ── 心跳每 30s ── Worker
  │                      │
  │    无响应 → 重启     │
  │    (max 3 次)        │
  │                      │
  ├──────────────────────┘
  │  超过 3 次 → 标记插件失效
```

---

## 14. 设计哲学

| 原则 | 说明 | 来源 |
|------|------|------|
| **YAGNI** | 不需要的功能不做、不加载 | Hermes + OpenClaw |
| **安全第一** | 默认拒绝，显式允许 | OpenClaw |
| **Prompt 缓存为王** | 所有决策以缓存命中为优先 | Claude Code |
| **模块隔离** | 每个模块有清晰的边界和接口 | Rust + OpenClaw |
| **崩溃是预期行为** | 系统应能从崩溃中恢复 | OpenClaw |
| **渐进式复杂性** | 简单的默认配置，按需启用高级功能 | Hermes |
| **SKILL.md 标准** | 技能格式与 Superpowers 兼容 | Superpowers |
| **Feature Flags** | 编译时裁剪，运行时零开销 | Claude Code |

---

## 15. 技术栈

| 层 | 技术 | 理由 |
|----|------|------|
| **语言** | Rust (2024 edition) | 内存安全、性能、类型系统 |
| **异步运行时** | Tokio + async-trait | Rust 生态标准 |
| **配置** | TOML + serde + .env | Rust 生态标准 |
| **数据库** | SQLite + sqlx + FTS5 | 零运维、全文搜索 |
| **IPC** | Unix Socket + JSON-RPC 2.0 | 简单、可调试 |
| **插件加载** | libloading | 跨平台 dylib 加载 |
| **TUI** | ratatui + crossterm | 活跃维护的 Rust TUI |
| **日志** | tracing + tracing-subscriber | 结构化日志 |
| **测试** | cargo test + tempdir | 内置测试框架 |
| **打包** | Cargo workspace | 模块化管理 |

### Feature Flags（完整列表）

```toml
[features]
default = ["tui", "memory-sqlite", "all-tools"]

# 核心模式
coordinator = []          # 多代理协调器模式
local-agent = []          # 本地独立代理

# UI
tui = ["dep:ratatui", "dep:crossterm"]

# Gateway
gateway = ["dep:tokio-tungstenite", "dep:axum"]

# 记忆后端
memory-sqlite = ["dep:rusqlite", "fts5"]
memory-file = []

# 远程
ssh-remote = ["dep:ssh2"]
tailscale = []

# Agent Swarm
agent-swarm = []

# 开发/调试
debug-mode = []           # 额外日志 + 诊断端点
benchmark = []            # 性能分析

# 内置工具（可选裁剪）
all-tools = ["tool-shell", "tool-file", "tool-git", "tool-browser", "tool-lsp", "tool-web"]
tool-shell = []
tool-file = []
tool-git = []
tool-browser = []
tool-lsp = []
tool-web = []
```

---

## 16. 测试策略

### 16.1 测试层级

| 层级 | 范围 | 运行频率 | 工具 |
|------|------|---------|------|
| 单元测试 | 单个函数 | 每次提交 | cargo test |
| 集成测试 | 模块间交互 | 每次提交 | cargo test --test |
| E2E 测试 | 完整系统 | 发版前 | 自定义测试脚本 |
| Live 测试 | 真实 LLM 调用 | 人工 | OPENCLAW_LIVE_TEST=1 |

### 16.2 测试原则

1. **Mock LLM API** — 测试不调用真实 API
2. **临时目录隔离** — 不污染 `~/.agent`
3. **不写 change-detector** — 测行为，不测具体值
4. **异步超时保护** — 所有 async 测试带 tokio::time::timeout
5. **确定性测试** — 相同输入相同输出

### 16.3 关键测试场景

```rust
#[tokio::test]
async fn test_tool_execute_parallel_safe() {
    let registry = setup_test_registry();
    let tool_calls = vec![
        ToolCall::new("read_file", json!({"path": "a.rs"})),
        ToolCall::new("read_file", json!({"path": "b.rs"})),
        ToolCall::new("read_file", json!({"path": "c.rs"})),
    ];
    let results = registry.execute_parallel(tool_calls).await;
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_worker_crash_recovery() {
    let pool = setup_worker_pool();
    pool.spawn("shell", shell_dylib()).await;
    crash_process(pool.get_pid("shell"));
    
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(pool.is_alive("shell")); // 自动重启
}

#[tokio::test]
async fn test_prompt_cache_hit() {
    let system_prompt = build_system_prompt(&mock_session()).await;
    let api_messages = vec![system_prompt.clone(), user_message()];
    let response = llm_client.chat(api_messages).await?;
    assert_eq!(response.cache_hit_tokens, system_prompt.len());
}
```

---

## 17. 安全模型

### 17.1 权限引擎

```
PermissionMode 控制工具执行：

┌─────────────┬──────────┬───────────┬───────────┐
│ 操作类型    │ auto     │ acceptEdits│ readOnly  │
├─────────────┼──────────┼───────────┼───────────┤
│ 文件读取    │ 允许     │ 允许      │ 允许      │
│ 文件写入    │ 询问     │ 允许      │ 拒绝      │
│ 执行命令    │ 询问     │ 允许(安全)│ 拒绝      │
│ 执行 Bash   │ 拒绝     │ 询问      │ 拒绝      │
│ 网络请求    │ 询问     │ 询问      │ 允许(读)  │
│ 删除文件    │ 拒绝     │ 询问      │ 拒绝      │
└─────────────┴──────────┴───────────┴───────────┘
```

### 17.2 文件路径策略

```rust
pub struct PathPolicy {
    pub always_allow: Vec<PathBuf>,     // ~/.hermes/ 等
    always_deny: Vec<PathBuf>,          // ~/.ssh/ 等
    allow_read: Vec<PathBuf>,           // workspace/
    allow_write: Vec<PathBuf>,          // workspace/
}

impl PathPolicy {
    pub fn check(&self, path: &Path, operation: FileMode) -> PermissionResult {
        // 1. 先检查 explicit allow/deny
        // 2. 再检查 read/write
        // 3. 默认拒绝
    }
}
```

### 17.3 沙箱模式（可选）

- **Docker**：工具在容器内执行
- **SSH**：远程沙箱执行（Feature: `ssh-remote`）
- **Seccomp**：Linux 系统调用限制

---

## 18. 实施路线图

### Phase 1: 核心基础（2-3 周）

- [x] Workspace 初始化
- [x] agent-config: TOML 配置管理
- [x] agent-tools: Tool trait + Registry
- [x] Built-in tools: shell, file, read/write
- [x] agent-llm: 单提供商（OpenAI-compatible）
- [x] agent-core: Agent Loop v1（同步模式）
- [x] CLI 入口

### Phase 2: 插件与 IPC（2-3 周）

- [x] agent-plugin-sdk: PluginRegistration + ToolBuilder
- [x] agent-worker: 进程启动、dylib 加载
- [x] agent-gateway: Unix Socket JSON-RPC
- [x] Plugin tool: git
- [x] 崩溃恢复机制

### Phase 3: 高级功能（2-3 周）

- [x] agent-memory: 内置文件记忆 + SQLite SessionDB
- [x] agent-skills: SKILL.md 解析 + Curator
- [x] ToolSets 过滤
- [x] agent-cron: 定时任务
- [x] agent-delegate: 委派/子代理
- [x] 权限引擎完整实现

### Phase 4: 平台与 UI（2-3 周）

- [x] agent-tui: ratatui TUI
- [x] agent-platforms: Telegram/Discord 适配器（各 1-2 个）
- [x] Prompt 缓存实现（Anthropic cache_control 或 OpenAI prefix）
- [x] 流式传输模式
- [x] Feature Flags 完整实现

### Phase 5: 扩展与优化（持续）

- [ ] 更多内置工具（browser, LSP, web_search）
- [ ] 更多平台适配器（Slack, Signal, WeChat）
- [ ] 技能市场（SKILL.md 包管理）
- [ ] 插件开发 SDK（文档 + 示例）
- [ ] 性能基准测试与优化
- [ ] 安全审计

---

## 附录 A: 命名规范

| 组件 | 命名 | 路径约定 |
|------|------|---------|
| 项目 | `agent-framework` | 仓库名 |
| Crates | `agent-xxx` | `crates/agent-xxx/` |
| 工具 dylib | `libxxx.so` | `tools/xxx/` |
| 技能 | `SKILL.md` | `skills/<category>/<name>/` |
| 配置 | `~/.agent/config.toml` | 用户级 |
| 状态 | `~/.agent/state/` | 用户级 |

## 附录 B: 错误分类

```rust
pub enum AgentError {
    // 工具错误
    ToolNotFound(String),
    ToolCrashed(String, String),          // (tool_name, stderr)
    ToolTimeout(String),
    ToolPermissionDenied(String),
    
    // LLM 错误
    LLMRateLimit,
    LLMContextTooLarge,
    LLMAuthenticationFailed,
    LLMServiceUnavailable,
    
    // 系统错误
    WorkerCrashed(String),
    PluginLoadFailed(String),
    DatabaseError(String),
    FileSystemError(String),
    
    // 用户可恢复
    UserCancelled,
    UserRejected,
}
```