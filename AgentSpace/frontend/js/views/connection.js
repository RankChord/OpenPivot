import { el, button, pageHeader, panel, copy } from '../ui.js';

// 显示工具职责，协议和参数的校验由 Rust MCP 适配层负责。
const methods = [
  ['agent_list / agent_get', '发现其他身份的职责、知识库与记忆路径'],
  ['file_list / file_read / file_write', '读取共享资料与私有文档，以版本号防止覆盖'],
  ['workflow_create / task_enqueue', '固定编排预建阶段；自动配合由执行 Agent 决定并派发下一项任务'],
  ['task_claim / task_list / task_get / task_cancel', '领取任务、查询状态，发起者撤回尚未执行的任务'],
  ['task_heartbeat / task_wait_on', '续租或等待依赖任务'],
  ['task_complete / task_fail', '提交产物、更新状态，并通知交付接收者'],
  ['inbox_poll / inbox_ack / message_send', '读取、确认与发送协作消息'],
];

/** 连接说明不包含管理员令牌。执行器凭注册 Agent 时取得的独立令牌调用 MCP。 */
export function renderConnection(ctx) {
  const url = new URL('/mcp', location.origin).href;
  const endpoint = el('div', { class: 'connection-body' },
    el('div', { class: 'connection-url' },
      el('code', { text: url }),
      button('复制地址', () => ctx.guard(() => copy(ctx, url)), 'button button-secondary')),
    el('div', { class: 'notice' }, '在「Agent 团队」注册身份并保存访问令牌，然后在客户端配置 Authorization: Bearer <Agent 令牌>。重新生成令牌会使旧令牌失效。'),
    connectionSteps());

  const tools = el('div', { class: 'table-scroll' },
    el('table', { class: 'data-table tool-table' },
      el('thead', {}, el('tr', {}, el('th', { text: 'MCP 工具' }), el('th', { text: '用途' }))),
      el('tbody', {}, methods.map(([names, description]) =>
        el('tr', {}, el('td', {}, el('code', { text: names })), el('td', { text: description }))))));

  return el('div', {},
    pageHeader('MCP CONNECTION', '连接真实 Agent', '工作区服务管理数据与协作状态，模型执行器连接 MCP 后负责实际任务处理。'),
    panel('远程 MCP 入口', 'HTTP 工具入口，执行端需要能访问这个地址。', endpoint,
      [button('管理身份与令牌 →', () => ctx.navigate('agents'), 'button button-small button-quiet')]),
    panel('工具与职责边界', '前端管理 API 和 MCP 工具共享同一套后端业务逻辑。', tools),
    el('div', { class: 'notice notice-warning' }, '本地 demo 的管理入口使用默认令牌。用于云端时，请设置独立管理密钥、配置 HTTPS，并按项目要求加入工作区隔离与执行环境。'));
}

function connectionSteps() {
  const steps = [
    ['注册稳定的执行身份', 'ID 对应固定队列与私有目录；模型标签记录当前客户端，可在更换模型时修改。'],
    ['让执行器接入 MCP', '把入口地址和独立令牌配置给支持远程 MCP 的客户端；跨设备协作需要部署为可访问的云端服务。'],
    ['运行任务轮询与执行循环', '领取 → 读取上下文 → 实际执行 → 持续续租 → 原子提交结果。自动配合时，执行 Agent 读取职责与产物，自行决定是否派发下一项任务；固定编排按预设阶段推进。任务与收件箱都需要执行器主动轮询。'],
  ];
  return el('ol', { class: 'connection-steps' }, steps.map(([title, description]) =>
    el('li', {}, el('strong', { text: title }), el('p', { text: description }))));
}
