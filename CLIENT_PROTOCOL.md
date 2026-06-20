# OpenPivot Client Protocol

本文档面向负责开发客户端的 Agent。目标是让客户端先接入当前 HTTP API，完成最小可用 IM 流程。

## 开发原则

客户端当前只需要支持 HTTP API。WebSocket 后续再接。

当前协议原则：

```text
HTTP API 负责写入事实和查询状态
WebSocket 后续只负责通知变化
```

不要把核心写操作设计成只能通过 WebSocket 完成。

## Base URL

本地默认：

```text
http://127.0.0.1:3000
```

API 前缀：

```text
/v1
```

## 通用请求规则

JSON 请求统一使用：

```http
Content-Type: application/json
```

需要登录态的接口统一携带：

```http
Authorization: Bearer <access_token>
```

客户端不要让用户手动输入 `user_id` 代表自己。当前用户身份必须来自 token。

## Token 生命周期

登录响应：

```json
{
  "access_token": "...",
  "refresh_token": "...",
  "token_type": "Bearer",
  "expires_in": 900
}
```

客户端建议：

```text
access_token: 存内存或安全存储，用于业务请求
refresh_token: 安全存储，用于刷新登录态
expires_in: 秒
```

当业务接口返回 `401 Unauthorized`：

```text
1. 调 POST /v1/auth/refresh
2. 成功则替换本地 token
3. 重试原请求一次
4. 刷新失败则回到登录页
```

## 错误格式

所有错误响应当前格式：

```json
{
  "code": "unauthorized",
  "message": "Unauthorized"
}
```

客户端应优先根据 HTTP 状态码处理：

```text
400: 请求参数不合法
401: 未登录或 access token 失效
403: 没有权限
409: 状态冲突，例如重复申请好友
500: 服务端错误
```

## 数据模型

### UserSearchItem / FriendItem

```json
{
  "id": 2,
  "username": "bob",
  "nickname": "Bob"
}
```

### FriendRequestResponse

```json
{
  "id": 1,
  "requester_id": 1,
  "addressee_id": 2,
  "status": "pending",
  "message": "你好"
}
```

status 目前可能值：

```text
pending
accepted
rejected
canceled
```

### ConversationResponse

```json
{
  "id": 1,
  "conversation_type": "direct",
  "user_low_id": 1,
  "user_high_id": 2
}
```

当前只支持：

```text
direct
```

### Message

```json
{
  "id": 1,
  "conversation_id": 1,
  "sender_id": 1,
  "content": "你好",
  "created_at": "2026-06-20T12:00:00Z"
}
```

## 认证接口

### Register

```http
POST /v1/auth/register
```

Request:

```json
{
  "username": "alice",
  "password": "password123",
  "nickname": "Alice"
}
```

Validation:

```text
username: 3-16 位，只允许 ASCII 英文数字
password: 至少 8 位
nickname: 1-64 位
```

### Login

```http
POST /v1/auth/login
```

Request:

```json
{
  "username": "alice",
  "password": "password123"
}
```

### Refresh

```http
POST /v1/auth/refresh
```

Request:

```json
{
  "refresh_token": "..."
}
```

### Logout

```http
POST /v1/auth/logout
```

Request:

```json
{
  "refresh_token": "..."
}
```

### Me

```http
GET /v1/auth/me
Authorization: Bearer <access_token>
```

Response:

```json
{
  "user_id": 1
}
```

## 用户与好友流程

### Search Users

```http
GET /v1/users/search?q=<keyword>
Authorization: Bearer <access_token>
```

客户端用途：搜索可添加用户。

### Create Friend Request

```http
POST /v1/friends/requests
Authorization: Bearer <access_token>
Content-Type: application/json
```

Request:

```json
{
  "user_id": 2,
  "message": "你好"
}
```

客户端约束：

```text
不要允许用户添加自己
message 可为空
重复申请时服务端会返回 409
```

### List Received Friend Requests

```http
GET /v1/friends/requests
Authorization: Bearer <access_token>
```

客户端用途：新的朋友页面、好友申请红点页面。

### Accept Friend Request

```http
POST /v1/friends/requests/:id/accept
Authorization: Bearer <access_token>
```

### Reject Friend Request

```http
POST /v1/friends/requests/:id/reject
Authorization: Bearer <access_token>
```

### List Friends

```http
GET /v1/friends
Authorization: Bearer <access_token>
```

客户端用途：通讯录、发起聊天前选择好友。

## 会话与消息流程

### Create Or Get Direct Conversation

```http
POST /v1/conversations/direct
Authorization: Bearer <access_token>
Content-Type: application/json
```

Request:

```json
{
  "user_id": 2
}
```

说明：

```text
user_id 是目标好友 id
只有好友之间允许创建 direct conversation
重复调用会返回同一个 direct conversation
```

### List Conversations

```http
GET /v1/conversations
Authorization: Bearer <access_token>
```

当前响应不包含最后一条消息和未读数，客户端第一版可以只展示会话 id 和对方 id。

### Send Message

```http
POST /v1/conversations/:id/messages
Authorization: Bearer <access_token>
Content-Type: application/json
```

Request:

```json
{
  "content": "你好"
}
```

规则：

```text
content trim 后不能为空
当前登录用户必须属于该 conversation
```

### List Messages

```http
GET /v1/conversations/:id/messages
Authorization: Bearer <access_token>
```

规则：

```text
当前登录用户必须属于该 conversation
当前返回最多 100 条
按 created_at ASC 排序
```

## 推荐客户端页面

第一版客户端建议只做这些页面：

```text
登录页
注册页
用户搜索页
好友申请页
好友列表页
会话列表页
聊天页
```

## 推荐状态管理

客户端至少维护：

```text
currentUserId
accessToken
refreshToken
friends
receivedFriendRequests
conversations
messagesByConversationId
```

## 最小端到端流程

```text
1. A 注册
2. B 注册
3. A 登录
4. B 登录
5. A 搜索 B
6. A 发送好友申请给 B
7. B 拉取收到的好友申请
8. B 同意申请
9. A 拉取好友列表
10. A 创建或获取和 B 的 direct conversation
11. A 发送消息
12. B 拉取 conversation 的消息
```

## WebSocket 预留设计

当前不要实现 WebSocket，但客户端架构可以预留事件入口。

未来事件可能是：

```json
{
  "type": "friend_request_created",
  "data": {
    "request_id": 1
  }
}
```

```json
{
  "type": "message_created",
  "data": {
    "conversation_id": 1,
    "message_id": 10
  }
}
```

客户端收到事件后不要完全依赖事件 payload 作为最终事实，建议重新拉取相关 HTTP API。

```text
friend_request_created -> GET /v1/friends/requests
message_created -> GET /v1/conversations/:id/messages
```

## 给客户端 Agent 的约束

开发客户端时请遵守：

```text
优先实现真实可用流程，不做营销页
先接 HTTP API，不做 WebSocket
不要假造后端不存在的字段
不要绕过 token 鉴权
不要在客户端信任用户自己传 sender_id
出现 401 时实现 refresh 流程
UI 上允许后端返回 400/403/409，并给出可理解提示
```
