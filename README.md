# OpenPivot

OpenPivot 是一个正在开发中的 IM 后端项目。当前阶段目标不是做完整产品，而是先打通最小可用链路：

```text
注册/登录 -> 搜索用户 -> 添加好友 -> 创建单聊会话 -> 发送消息 -> 拉取消息
```

项目当前使用 Rust + Axum + PostgreSQL + SQLx。

## 当前进度

已经完成的能力：

```text
认证：
- 注册
- 登录
- access token
- refresh token
- logout
- me

用户：
- 搜索用户

好友：
- 发送好友申请
- 查看收到的待处理申请
- 同意好友申请
- 拒绝好友申请
- 查看好友列表

会话与消息：
- 创建或获取单聊会话
- 查看我的会话列表
- 发送消息
- 拉取消息
```

当前主要还没有做：

```text
- WebSocket 实时通知
- 未读数
- 已读回执
- 消息分页
- 撤回/删除消息
- 群聊
- 文件/图片消息
- 更细粒度的错误码
- 自动化测试
```

## 启动方式

第一次启动时，如果配置文件不存在，程序会创建默认配置并退出。

配置文件路径：

```text
~/.config/openpivot/openpivot.conf
```

默认配置创建后，需要把：

```toml
[app]
enable = true
```

数据库配置使用拆分字段，而不是单个 URL：

```toml
[database]
host = "127.0.0.1"
port = 5432
username = "openpivot"
password = "openpivot"
database = "openpivot"
max_connections = 10
```

启动：

```bash
cargo run
```

检查编译：

```bash
cargo check
```

## 数据库迁移

迁移文件位于：

```text
migrations/
```

当前包含：

```text
202606070001_create_users.sql
202606070002_create_user_sessions.sql
202606140001_create_friend_requests.sql
202606140002_create_friendships.sql
202606140003_create_conversations.sql
202606140004_create_messages.sql
```

程序启动时会执行 SQLx migration。

## 鉴权方式

除注册、登录、刷新 token 等接口外，业务接口通常需要 access token。

请求头格式：

```http
Authorization: Bearer <access_token>
```

access token 过期后，客户端应使用 refresh token 调用刷新接口获取新 token。

## API 概览

基础路径：

```text
/v1
```

### Auth

#### 注册

```http
POST /v1/auth/register
Content-Type: application/json
```

请求：

```json
{
  "username": "alice",
  "password": "password123",
  "nickname": "Alice"
}
```

规则：

```text
username: 3-16 位，只允许英文和数字
password: 至少 8 位
nickname: 1-64 位
```

响应：

```json
{
  "id": 1,
  "username": "alice",
  "nickname": "Alice"
}
```

#### 登录

```http
POST /v1/auth/login
Content-Type: application/json
```

请求：

```json
{
  "username": "alice",
  "password": "password123"
}
```

响应：

```json
{
  "access_token": "...",
  "refresh_token": "...",
  "token_type": "Bearer",
  "expires_in": 900
}
```

#### 刷新 Token

```http
POST /v1/auth/refresh
Content-Type: application/json
```

请求：

```json
{
  "refresh_token": "..."
}
```

#### 登出

```http
POST /v1/auth/logout
Content-Type: application/json
```

请求：

```json
{
  "refresh_token": "..."
}
```

成功响应：

```text
204 No Content
```

#### 当前用户

```http
GET /v1/auth/me
Authorization: Bearer <access_token>
```

响应：

```json
{
  "user_id": 1
}
```

### Users

#### 搜索用户

```http
GET /v1/users/search?q=ali
Authorization: Bearer <access_token>
```

响应：

```json
[
  {
    "id": 2,
    "username": "alice2",
    "nickname": "Alice Two"
  }
]
```

### Friends

#### 好友列表

```http
GET /v1/friends
Authorization: Bearer <access_token>
```

响应：

```json
[
  {
    "id": 2,
    "username": "bob",
    "nickname": "Bob"
  }
]
```

#### 发送好友申请

```http
POST /v1/friends/requests
Authorization: Bearer <access_token>
Content-Type: application/json
```

请求：

```json
{
  "user_id": 2,
  "message": "你好"
}
```

响应：

```json
{
  "id": 1,
  "requester_id": 1,
  "addressee_id": 2,
  "status": "pending",
  "message": "你好"
}
```

#### 查看收到的待处理好友申请

```http
GET /v1/friends/requests
Authorization: Bearer <access_token>
```

#### 同意好友申请

```http
POST /v1/friends/requests/:id/accept
Authorization: Bearer <access_token>
```

#### 拒绝好友申请

```http
POST /v1/friends/requests/:id/reject
Authorization: Bearer <access_token>
```

### Conversations

#### 创建或获取单聊会话

```http
POST /v1/conversations/direct
Authorization: Bearer <access_token>
Content-Type: application/json
```

请求：

```json
{
  "user_id": 2
}
```

响应：

```json
{
  "id": 1,
  "conversation_type": "direct",
  "user_low_id": 1,
  "user_high_id": 2
}
```

只有好友之间允许创建 direct conversation。

#### 查看我的会话列表

```http
GET /v1/conversations
Authorization: Bearer <access_token>
```

#### 发送消息

```http
POST /v1/conversations/:id/messages
Authorization: Bearer <access_token>
Content-Type: application/json
```

请求：

```json
{
  "content": "你好"
}
```

响应：

```json
{
  "id": 1,
  "conversation_id": 1,
  "sender_id": 1,
  "content": "你好",
  "created_at": "2026-06-20T12:00:00Z"
}
```

#### 拉取消息

```http
GET /v1/conversations/:id/messages
Authorization: Bearer <access_token>
```

当前按创建时间正序返回最多 100 条。

## 错误响应

统一错误响应格式：

```json
{
  "code": "bad_request",
  "message": "Bad request"
}
```

当前常见错误：

```text
400 Bad Request
401 Unauthorized
403 Forbidden
409 Conflict
500 Internal Server Error
```

## 客户端建议

当前客户端开发可以先按 HTTP 拉取模型实现，不需要一开始接 WebSocket。

建议流程：

```text
登录后保存 access_token 和 refresh_token
所有业务请求带 Authorization header
access token 过期时调用 refresh
好友申请列表通过 GET /v1/friends/requests 拉取
消息通过 GET /v1/conversations/:id/messages 拉取
```

后续 WebSocket 的定位：

```text
HTTP API: 创建事实、执行操作、返回结果
WebSocket: 通知客户端有变化
```

## Agent 协作约定

本项目有开发协作偏好文件：

```text
agent.md
```

后续 Agent 参与开发时，应先阅读该文件。核心原则是：不要直接替项目作者大规模修改业务代码，优先给结构、示例、解释和审查。
