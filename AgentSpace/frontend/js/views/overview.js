import { el, button, pageHeader, panel, empty, statusBadge, when, agentName } from '../ui.js';
import { editAgent } from './agents.js';
import { enqueueTask, showTask } from './tasks.js';

/** 总览使用同一份 snapshot，不分别拉取统计，避免同一屏出现不同时间点的计数。 */
export function renderOverview(ctx) {
  const stats = ctx.state.stats;
  const metrics = [
    ['注册 Agent', stats.agents ?? ctx.state.agents.length, '职责与记忆保持独立', 'indigo'],
    ['等待领取', stats.queued || 0, '等待执行器领取任务', 'orange'],
    ['执行 / 依赖中', (stats.leased || 0) + (stats.blocked || 0), '租约保护与任务交接', 'teal'],
    ['已完成交付', stats.completed || 0, `${stats.files || 0} 个工作区文件`, 'purple'],
  ];
  const metricsNode = el('div', { class: 'metric-grid' }, metrics.map(([label, value, detail, color]) => el('div', { class: `metric-card metric-${color}` }, el('span', { class: 'metric-label', text: label }), el('strong', { class: 'metric-value', text: value }), el('span', { class: 'metric-detail', text: detail }))));
  const recent = [...ctx.state.tasks].sort((a, b) => b.updated_at - a.updated_at).slice(0, 6);
  const rows = el('div', { class: 'recent-list' }, recent.map(task => taskRow(ctx, task)));
  const team = el('div', { class: 'team-list' }, ctx.state.agents.slice(0, 5).map(agent => el('div', { class: 'team-row' }, el('span', { class: 'mini-avatar', text: agent.name.slice(0, 1) }), el('div', {}, el('strong', { text: agent.name }), el('span', { text: agent.responsibility })), el('span', { class: `dot ${agent.active ? 'dot-active' : ''}`, title: agent.active ? '已启用' : '已停用' }))));
  return el('div', {}, pageHeader('WORKSPACE OVERVIEW', '让交付在同一个空间流转', '通过 MCP 共享资料和消息，以任务队列连接各个 Agent 的执行流程。', [button('创建任务 →', () => enqueueTask(ctx), 'button button-primary', { disabled: !ctx.state.agents.some(a => a.active) })]), metricsNode, el('div', { class: 'overview-grid' }, panel('最近任务', '从指派、执行到验收，跟踪每一次交接。', recent.length ? rows : empty('还没有任务', '注册 Agent 后，将需求加入它的任务队列。', button('安排团队', () => ctx.navigate('agents'))), [button('全部任务 →', () => ctx.navigate('tasks'), 'button button-small button-quiet')]), panel('你的 Agent 团队', `${ctx.state.agents.filter(a => a.active).length} 个启用身份`, team.children.length ? team : empty('团队由你定义', '无需预置四个角色；从实际需要开始。', button('注册 Agent', () => editAgent(ctx))), [button('管理团队 →', () => ctx.navigate('agents'), 'button button-small button-quiet')])), el('section', { class: 'workflow-strip' }, el('div', {}, el('span', { class: 'eyebrow', text: 'HOW WORK MOVES' }), el('h2', { text: '从任务到交付的一条协作链' })), el('div', { class: 'workflow-steps' }, [['01', '定义职责', '注册稳定身份'], ['02', '指派任务', '进入目标队列'], ['03', '执行与协作', '索取资料，等待依赖'], ['04', '提交与通知', '成果版本与消息一起交付']].map(([number, title, description]) => el('div', { class: 'workflow-step' }, el('span', { text: number }), el('strong', { text: title }), el('small', { text: description })))), el('p', { class: 'helper', text: '控制台可以模拟执行器领取和提交任务；真实模型客户端需要连接 MCP 并运行自己的轮询与执行循环。' })));
}

function taskRow(ctx, task) {
  return el('button', { type: 'button', class: 'recent-row', onclick: () => showTask(ctx, task) }, el('div', { class: 'recent-title' }, el('strong', { text: task.title }), el('span', { text: `${agentName(ctx, task.assignee)} · ${when(task.updated_at)}` })), statusBadge(task.status));
}
