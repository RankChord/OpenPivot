# Agent Framework — 快速运行指南

## 1. 编译项目

```bash
cd /home/actions/reserch
cargo build --workspace
```

## 2. 运行测试

```bash
cargo test --workspace
```

这会运行所有 crate 的测试：
- `agent-config` (3 tests) — 配置系统，TOML 解析
- `agent-tools` (4 tests) — 工具注册表，执行，generation counter
- `agent-llm` (2 tests) — LLM 客户端构建
- `agent-core` (3 tests) — 预算控制，消息构建
- `agent-skills` (1 test) — SKILL.md 解析
- `agent-cron` (5 tests) — 调度器格式解析
- `agent-delegate` (6 tests) — 委派系统（深度/并发控制）

## 3. 运行 CLI

```bash
cargo run -p agent-cli -- --help
```

你会看到：
```
AI Agent Framework

Usage: agent [OPTIONS] [COMMAND]

Commands:
  run     Start interactive agent
  config  Show configuration
  help    Print this message or the help of the given subcommand(s)
```

## 4. 查看配置（无需 API Key）

```bash
cargo run -p agent-cli -- config
```

输出：
```
Config file not found at /home/actions/.agent/config.toml
Using defaults:
  Model: openai - gpt-4
```

## 5. 创建配置文件（可选）

```bash
mkdir -p ~/.agent
cat > ~/.agent/config.toml << 'EOF'
[model]
provider = "openai"
name = "gpt-4"
base_url = "https://api.openai.com"
# api_key 从环境变量 AGENT_API_KEY 读取

[agent]
max_iterations = 50
stream_mode = "full"
auto_approve_tools = false
EOF
```

再运行 `cargo run -p agent-cli -- config` 会显示你配置的内容。

## 6. 构建工具 dylib

所有工具已经在 workspace 中了。你可以单独构建某个工具：

```bash
cargo build -p tool-read-file    # → target/debug/libread_file.so (或 .dylib)
cargo build -p tool-write-file
cargo build -p tool-shell
cargo build -p tool-todo
cargo build -p tool-web-search
```

## 7. 完整项目结构速查

```
agent-framework/
├── Cargo.toml              ← workspace 根
├── crates/                 ← 16 个核心 crate
│   ├── agent-config        ← TOML 配置
│   ├── agent-tools         ← Tool trait + Registry
│   ├── agent-llm           ← 多提供商 LLM (OpenAI/Anthropic/Ollama + 流式)
│   ├── agent-core          ← Agent Loop + Budget
│   ├── agent-cli           ← CLI 入口
│   ├── agent-gateway       ← JSON-RPC over Unix Socket
│   ├── agent-worker        ← 进程池管理
│   ├── agent-plugin-sdk    ← 纯 trait SDK (零运行时依赖)
│   ├── agent-memory        ← 内存系统 (文件 + 提供者)
│   ├── agent-skills        ← SKILL.md 引擎 + Curator
│   ├── agent-permission    ← 路径安全策略
│   ├── agent-cron          ← 定时任务调度器
│   ├── agent-delegate      ← 子代理/委派系统
│   ├── agent-tui           ← ratatui 终端 UI
│   ├── agent-sessions      ← SQLite + FTS5 会话存储
│   └── agent-platforms     ← 平台适配器 (Telegram/Discord)
└── tools/                  ← 5 个工具 dylib
    ├── read-file           ← 文件读取 (cdylib)
    ├── write-file          ← 文件写入 (cdylib)
    ├── shell               ← Shell 命令 (cdylib)
    ├── todo-tool           ← 待办清单 (cdylib)
    └── web-search          ← 网络搜索 (cdylib)
```

## 8. 下一步

要实现完整的工具链闭环，需要：
1. **设置 API Key**: `export AGENT_API_KEY=your_key_here`
2. **配置正确的 base_url**: 编辑 `~/.agent/config.toml`
3. **运行**: `cargo run -p agent-cli -- run "你好，请介绍一下你自己"`
4. **TUI**: `cargo run -p agent-tui`（需要终端支持）
