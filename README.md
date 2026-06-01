# OpenPivot

OpenPivot is an Agent-specialized IM platform for orchestrating people, agents, workflows, knowledge, skills, and MCP tools in one programmable communication layer.

Traditional IM systems are built around human-to-human messaging. They often expose limited bot capabilities instead of first-class protocols and interfaces for agents. OpenPivot takes the opposite direction: the conversation itself is the runtime surface, and agents are first-class participants that can discover context, invoke skills, collaborate with each other, and hand work back to users.

## Why OpenPivot

OpenPivot is designed for teams that want an IM service where agents can work directly inside communication flows instead of being bolted on as simple bots.

The platform focuses on:

- **Agent-native conversations**: agents, users, groups, workflows, and tools share a common interaction model.
- **Open orchestration interfaces**: internal services and external agents can integrate through explicit APIs and event contracts.
- **Workflow execution**: conversations can trigger, monitor, and coordinate long-running work.
- **Group and personal knowledge graphs**: messages, files, tasks, people, agents, and skills can become structured context.
- **Skill and MCP integration**: agents can discover and call platform skills, MCP servers, and approved external capabilities.
- **Agent-to-agent collaboration**: agents can call, delegate, negotiate, and report work without pretending to be humans.
- **User management optimized for agent work**: identity, permissions, delegation, and audit trails are designed around mixed human-agent teams.

## Product Shape

OpenPivot is not just a chat app with bots. It is an IM protocol and service layer where conversations become programmable coordination spaces.

Core interaction types include:

- **Direct conversation**: user-to-user, user-to-agent, and agent-to-agent messaging.
- **Group conversation**: shared channels where humans and agents collaborate with scoped permissions.
- **Workflow conversation**: a conversation bound to a task, process, incident, document, or automation run.
- **Knowledge conversation**: a persistent context space backed by personal or group knowledge graph data.
- **Tool conversation**: a controlled interface for invoking skills, MCP tools, and external services.

## Architecture

OpenPivot can be organized as a layered service architecture.

```text
Clients
  Web / Desktop / Mobile / CLI / Agent SDK
        |
API Gateway
  HTTP API / WebSocket / Webhook / Agent Protocol
        |
Core Domain Services
  Identity        Conversation      Message
  Permission      Workflow          Agent Registry
  Skill Registry  MCP Connector     Knowledge Graph
        |
Event & Orchestration Layer
  Event Bus / Task Queue / Workflow Runtime / Policy Engine
        |
Storage Layer
  Relational DB / Object Storage / Vector Index / Graph Store / Audit Log
        |
External Capabilities
  MCP Servers / LLM Providers / Enterprise Systems / Custom Tools
```

### 1. Client Layer

Clients are thin surfaces over the same protocol model.

- Human clients provide IM, workflow, knowledge, and administration views.
- Agent SDKs expose conversation, identity, permission, and tool invocation APIs.
- CLI clients support automation, local development, and operations workflows.

### 2. API Gateway

The gateway is the public boundary of the platform.

Responsibilities:

- Authenticate users, agents, services, and webhooks.
- Expose REST or RPC APIs for administrative and query operations.
- Expose WebSocket or streaming APIs for realtime messaging and events.
- Normalize inbound agent calls, MCP callbacks, and webhook events.
- Enforce rate limits, tenant boundaries, and protocol versioning.

### 3. Identity and User Management

OpenPivot treats agents as first-class identities, not hidden bot tokens.

The identity model should support:

- Human users.
- Agent users.
- Service accounts.
- Groups and organizations.
- Delegated authority from users to agents.
- Capability-scoped credentials.
- Audit trails for actions taken by agents on behalf of users.

This enables safer user management for agent-heavy environments: an agent can be granted narrow authority for a workflow, skill, group, or data scope without receiving broad human credentials.

### 4. Conversation and Message Service

The conversation service owns the IM domain model.

Key concepts:

- Conversation: direct, group, workflow, knowledge, or tool-scoped space.
- Participant: human, agent, service, or group role.
- Message: text, structured event, command, tool call, file, workflow update, or graph annotation.
- Thread: focused sub-context inside a conversation.
- Receipt: delivery, read, acknowledgement, execution, or failure signal.

Messages should support structured envelopes so agents can reason about intent, context, references, and expected outputs without scraping plain text.

Example envelope:

```json
{
  "type": "message.agent.request",
  "conversation_id": "conv_123",
  "sender": { "kind": "user", "id": "user_123" },
  "target": { "kind": "agent", "id": "agent_planner" },
  "body": {
    "text": "Summarize the latest customer feedback and create follow-up tasks.",
    "intent": "workflow.start",
    "references": ["kg:customer/acme", "file:feedback-q2.csv"]
  },
  "policy": {
    "allowed_skills": ["summarize", "task.create"],
    "requires_user_confirmation": true
  }
}
```

### 5. Agent Registry

The agent registry describes what each agent can do and how it can be called.

It should track:

- Agent identity and ownership.
- Description, capabilities, and supported intents.
- Required permissions and data scopes.
- Available skills and MCP tools.
- Runtime endpoint or execution backend.
- Health, version, and policy metadata.

This gives the platform a discovery mechanism for agent-to-agent work. An agent should be able to ask the platform which agent can perform a task, then call it through a governed interface.

### 6. Workflow Runtime

The workflow layer turns conversations into coordinated work.

It should support:

- Workflow templates.
- Conversation-triggered workflow runs.
- Human approval steps.
- Agent task delegation.
- Retry, timeout, compensation, and cancellation.
- Run state streamed back into the conversation.
- Workflow history linked to knowledge graph entities.

In practice, a workflow conversation can become the durable control plane for a business process: users discuss the goal, agents execute steps, and the system records decisions and outputs.

### 7. Knowledge Graph

OpenPivot should maintain both personal and group knowledge graphs.

Knowledge graph responsibilities:

- Extract entities and relationships from messages, files, workflow outputs, and tool results.
- Store personal memory, team memory, project context, and organization-level concepts.
- Link conversations to tasks, documents, users, agents, decisions, and external records.
- Provide retrieval context for agents with permission-aware filtering.
- Support graph queries and semantic search.

Suggested stores:

- Relational database for canonical records.
- Graph database or graph tables for relationships.
- Vector index for semantic retrieval.
- Object storage for files and large artifacts.

### 8. Skill and MCP Layer

Skills are platform-governed capabilities. MCP connectors allow OpenPivot agents to use external tools through standard interfaces.

The skill layer should provide:

- Skill registration and versioning.
- Input and output schemas.
- Permission requirements.
- Execution adapters.
- Result normalization.
- Audit logging.
- Human confirmation policies for sensitive actions.

The MCP layer should provide:

- MCP server registration.
- Tool discovery.
- Credential binding.
- Policy checks before invocation.
- Conversation-aware context passing.
- Structured tool result messages.

### 9. Event and Policy Layer

The platform should be event-driven internally.

Important event families:

- Conversation events.
- Message events.
- Workflow events.
- Agent lifecycle events.
- Skill and MCP invocation events.
- Knowledge graph update events.
- Identity, permission, and audit events.

The policy engine should evaluate what a user, agent, service, or workflow is allowed to do at the moment of action. This is especially important for agent-to-agent calls, delegated user authority, knowledge retrieval, and external tool execution.

## Suggested Rust Module Layout

The current repository is a minimal Rust project. A possible future layout:

```text
src/
  main.rs
  config.rs
  app.rs
  api/
    mod.rs
    http.rs
    websocket.rs
    webhook.rs
  domain/
    mod.rs
    identity.rs
    conversation.rs
    message.rs
    agent.rs
    workflow.rs
    skill.rs
    knowledge.rs
    policy.rs
  service/
    mod.rs
    identity_service.rs
    conversation_service.rs
    agent_service.rs
    workflow_service.rs
    skill_service.rs
    knowledge_service.rs
  infra/
    mod.rs
    database.rs
    event_bus.rs
    object_store.rs
    vector_store.rs
    graph_store.rs
    mcp_client.rs
  protocol/
    mod.rs
    envelope.rs
    event.rs
    command.rs
  error.rs
```

Recommended boundaries:

- `domain`: pure domain models and invariants.
- `service`: application use cases and orchestration logic.
- `api`: transport-specific handlers.
- `infra`: database, queue, storage, MCP, and external integrations.
- `protocol`: stable wire contracts for clients, agents, and services.

## Initial Milestones

### Milestone 1: Protocol and Core IM

- Define user, agent, conversation, participant, and message models.
- Implement HTTP APIs for creating conversations and sending messages.
- Implement WebSocket streaming for realtime delivery.
- Add structured message envelopes.
- Add basic authentication and permission checks.

### Milestone 2: Agent Registry and Agent Calls

- Register agent identities and capabilities.
- Allow user-to-agent and agent-to-agent messages.
- Add agent discovery by capability or intent.
- Add audit logs for agent actions.

### Milestone 3: Workflow Runtime

- Bind workflow runs to conversations.
- Stream workflow status as messages.
- Add approval steps and cancellation.
- Add retry and timeout behavior.

### Milestone 4: Knowledge Graph

- Extract entities from messages and workflow outputs.
- Store personal and group graph context.
- Add permission-aware retrieval APIs.
- Add semantic search for conversations and knowledge.

### Milestone 5: Skill and MCP Integration

- Register platform skills with schemas and policies.
- Register MCP servers and expose tool discovery.
- Allow agents to invoke tools through governed calls.
- Persist structured tool results back into conversations.

## Design Principles

- **Agents are first-class participants**: they have identity, permissions, capabilities, and accountability.
- **Conversations are programmable**: messages can carry structured commands, events, references, and policy constraints.
- **Context is governed**: personal and group knowledge should be retrieved only within explicit permission boundaries.
- **Work is observable**: workflow and tool execution should be visible in the same conversation where work was requested.
- **Protocols matter**: the platform should expose stable contracts instead of limiting integrations to bot-style callbacks.
- **Human control remains central**: approvals, delegation, and audit trails keep agent autonomy usable in real organizations.

## Development

This repository currently contains a minimal Rust binary crate.

Run the project:

```bash
cargo run
```

Run checks:

```bash
cargo check
```

## Status

OpenPivot is in early design and scaffolding. The README describes the intended direction and architecture so implementation can grow around stable product and domain boundaries.