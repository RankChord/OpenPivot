import { api } from '../api.js';
import { el, button, field, input, textarea, pageHeader, empty, modal, submit, copy } from '../ui.js';

/** 新建与密钥轮换只在此次响应中显示密钥；刷新后不会从服务端重新读取。 */
export function showCredentials(ctx, agent, token) {
  const url = new URL('/mcp', location.origin).href;
  const config = JSON.stringify({ mcpServers: { agentspace: { url, headers: { Authorization: `Bearer ${token}` } } } }, null, 2);
  const body = el('div', { class: 'modal-body' }, el('p', { class: 'notice notice-warning', text: `请现在保存 ${agent.name} 的访问令牌。它只显示这一次，后续可重新生成。` }), field('MCP 服务地址', input('mcp-url', url, { readOnly: true })), field('Agent 访问令牌', input('agent-token', token, { readOnly: true })), el('pre', { class: 'code-block', text: config }), el('p', { class: 'helper', text: '这是远程 HTTP MCP 连接示例。按实际客户端的配置格式填入服务地址和 Authorization 请求头。' }), el('div', { class: 'form-actions' }, button('复制令牌', () => ctx.guard(() => copy(ctx, token))), button('复制 MCP 配置', () => ctx.guard(() => copy(ctx, config)), 'button button-primary')));
  modal('Agent 已准备好连接', body, true);
}

export function editAgent(ctx, agent = null) {
  const form = el('form', { class: 'modal-body' });
  const id = input('id', agent?.id || '', { required: true, maxLength: 64, pattern: '[a-z0-9][a-z0-9_-]{0,63}', placeholder: '例如 backend 或 planner', readOnly: Boolean(agent) });
  const name = input('name', agent?.name || '', { required: true, maxLength: 120, placeholder: '例如 后端工程师' });
  const responsibility = textarea('responsibility', agent?.responsibility || '', { required: true, maxLength: 8000, placeholder: '明确它负责什么、交付什么、向谁汇报。' });
  const model = input('model', agent?.model || '', { maxLength: 200, placeholder: '例如 Codex / WorkBuddy / 自建模型，可留空' });
  form.append(field('稳定身份 ID', id, '小写字母、数字、下划线和短横线；创建后保持稳定，更换模型不会迁移知识与记忆。'), field('显示名称', name), field('职责与交付要求', responsibility), field('模型 / 客户端标签', model), el('p', { class: 'helper', text: '此标签用于记录当前执行器。注册身份不会自动启动模型，执行器需自行连接 MCP 并轮询队列。' }));
  const dialog = modal(agent ? '编辑 Agent' : '注册新 Agent', form);
  form.append(el('div', { class: 'form-actions' }, button('取消', dialog.close), el('button', { type: 'submit', class: 'button button-primary', text: agent ? '保存修改' : '注册并生成令牌' })));
  form.addEventListener('submit', event => {
    event.preventDefault();
    submit(form, async () => {
      const values = { name: name.value.trim(), responsibility: responsibility.value.trim(), model: model.value.trim() };
      if (agent) {
        await api.updateAgent(agent.id, { ...values, active: agent.active });
        dialog.close(); await ctx.refreshAfterWrite(); ctx.toast('Agent 职责已更新');
      } else {
        const result = await api.createAgent({ id: id.value.trim(), ...values });
        dialog.close(); await ctx.refreshAfterWrite(); ctx.state.actor = result.agent.id; showCredentials(ctx, result.agent, result.token);
      }
    });
  });
}

function card(ctx, agent, index) {
  const queued = ctx.state.tasks.filter(task => task.assignee === agent.id && task.status === 'queued').length;
  const colors = ['indigo', 'teal', 'orange', 'purple'];
  const actions = el('div', { class: 'agent-actions' }, button('编辑职责', () => editAgent(ctx, agent), 'button button-small button-secondary'), button('知识与记忆', () => { ctx.state.fileActor = ''; ctx.state.filePath = null; ctx.state.fileVersion = null; ctx.state.filePrefix = agent.memory_prefix; ctx.navigate('files'); }, 'button button-small button-quiet'), button('连接配置', () => {
    const body = el('div', { class: 'modal-body' }, el('p', { text: '已发放的令牌无法读取。重新生成会立即使旧令牌失效，请随后更新执行器配置。' }));
    const dialog = modal('重新生成访问令牌', body);
    body.append(el('div', { class: 'form-actions' }, button('取消', dialog.close), button('重新生成并查看', () => ctx.guard(async () => { const result = await api.rotateToken(agent.id); dialog.close(); showCredentials(ctx, result.agent, result.token); }), 'button button-primary')));
  }, 'button button-small button-quiet', { disabled: !agent.active }), button(agent.active ? '停用' : '启用', () => ctx.guard(async () => {
    await api.updateAgent(agent.id, { name: agent.name, responsibility: agent.responsibility, model: agent.model || '', active: !agent.active });
    await ctx.refreshAfterWrite(); ctx.toast(agent.active ? 'Agent 已停用，历史和文件保留' : 'Agent 已启用');
  }), 'button button-small button-quiet'));
  return el('article', { class: `agent-card${agent.active ? '' : ' agent-inactive'}` }, el('div', { class: 'agent-card-top' }, el('div', { class: `avatar avatar-${colors[index % colors.length]}`, text: agent.name.slice(0, 1) }), el('span', { class: `badge ${agent.active ? 'badge-success' : 'badge-muted'}`, text: agent.active ? '已启用' : '已停用' })), el('h2', { text: agent.name }), el('code', { class: 'agent-id', text: agent.id }), el('p', { class: 'agent-responsibility', text: agent.responsibility }), el('div', { class: 'agent-meta' }, el('span', { text: agent.model || '执行器待连接' }), el('strong', { text: `${queued} 项待领取` })), el('div', { class: 'agent-paths' }, el('span', { text: `知识 ${agent.knowledge_prefix}` }), el('span', { text: `记忆 ${agent.memory_prefix}` })), actions);
}

export function renderAgents(ctx) {
  return el('div', {}, pageHeader('TEAM DIRECTORY', '安排你的 Agent 团队', '按项目需要注册任意数量的稳定身份，每个身份拥有独立职责、队列、知识库与记忆。', [button('＋ 注册 Agent', () => editAgent(ctx), 'button button-primary')]), ctx.state.agents.length ? el('div', { class: 'agent-grid' }, ctx.state.agents.map((agent, index) => card(ctx, agent, index))) : empty('从第一个 Agent 开始', '可以只安排一个 Agent，也可以按开发、设计、测试、运维等职责逐步扩展。', button('注册第一个 Agent', () => editAgent(ctx), 'button button-primary')), el('div', { class: 'notice' }, el('strong', { text: '身份随项目，模型可更换。 ' }), '职责通过 agent_list / agent_get 提供给所有执行器；私有知识与记忆只允许身份本身或管理员访问。停用会保留历史，有未完成任务时需要先处理任务。'));
}
