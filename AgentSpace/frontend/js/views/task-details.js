import { el, button, agentName, statusBadge, when, copy } from '../ui.js';
import { coordinationMode, coordinationLabel } from '../task-graph-model.js';

/**
 * 图形节点与原有任务弹窗共享同一份详情渲染。
 * 详情只读任务快照；文件链接携带交付版本，防止误读后来更新的版本。
 * 导航/派单由外层回调处理，本模块不持有页面路由或弹窗生命周期。
 */
export function renderTaskDetails(ctx, task, options = {}) {
  const { className = 'modal-body', onLeave = () => {}, onSelect, onDelegate } = options;
  const owner = ctx.state.agents.find(agent => agent.id === task.assignee);
  const name = agentName(ctx, task.assignee);
  const mode = coordinationMode(task);
  const body = el('div', { class: className, 'data-task-id': task.id },
    el('div', { class: 'task-detail-owner' },
      el('span', { class: `avatar avatar-indigo task-owner-avatar owner-${task.status}`, 'aria-hidden': true, text: name.slice(0, 1) }),
      el('div', {}, el('strong', { text: name }), el('p', { class: 'helper', text: owner?.model || task.assignee })),
      statusBadge(task.status)),
    el('h3', { class: 'task-detail-title', text: task.title }),
    el('code', { class: 'task-full-id', text: task.id }),
    el('dl', { class: 'detail-grid' },
      detail('协作模式', el('span', { class: `badge mode-${mode}`, text: coordinationLabel(task) })),
      detail('发起者', agentName(ctx, task.sender)),
      detail('结果通知', agentName(ctx, task.reply_to)),
      detail('尝试次数', `${task.attempt} / ${task.max_attempts}`),
      detail('最近更新', when(task.updated_at))));

  body.append(el('p', { class: 'task-mode-explanation', text: mode === 'fixed'
    ? '固定编排：执行路线已在创建时定义。前一阶段完成后，后续阶段按顺序进入队列；不在此流程中临时派发新阶段。'
    : 'Agent 自动配合：后续协作者由执行 Agent 根据目标或工作结果决定。图形展示已经发生的派发；客户端仍需通过 MCP 轮询领取任务。' }));
  if (task.workflow_id) body.append(el('p', { class: 'helper task-workflow-id' }, '流程 ID：', el('code', { text: task.workflow_id })));

  if (task.lease_until) body.append(el('p', { class: 'notice', text: `执行租约到期：${when(task.lease_until)}` }));
  if (task.last_error) body.append(el('p', { class: 'notice notice-error', text: task.last_error }));
  if (task.parent_task_id) body.append(relatedTask(ctx, '上级任务', task.parent_task_id, onSelect));
  if (task.depends_on) body.append(relatedTask(ctx, '执行依赖', task.depends_on, onSelect));

  body.append(el('div', { class: 'task-detail-section-heading' },
    el('h4', { text: '任务提示词' }),
    button('复制提示词', () => ctx.guard(() => copy(ctx, task.instructions)), 'button button-small button-quiet')),
  el('pre', { class: 'prose-block task-prompt', text: task.instructions }));

  const summary = task.result?.summary || task.summary;
  if (summary) body.append(el('h4', { text: '交付摘要' }), el('pre', { class: 'prose-block', text: summary }));
  const artifacts = task.result?.artifacts || [];
  body.append(el('h4', { text: `交付文件 · ${artifacts.length}` }));
  if (artifacts.length) {
    body.append(el('div', { class: 'artifact-list' }, artifacts.map(artifact =>
      button(`${artifact.path} · v${artifact.version}`, () => {
        ctx.state.fileActor = '';
        ctx.state.filePrefix = artifact.path.slice(0, artifact.path.lastIndexOf('/') + 1);
        ctx.state.filePath = artifact.path;
        ctx.state.fileVersion = artifact.version;
        onLeave(); ctx.navigate('files');
      }, 'artifact-link'))));
  } else body.append(el('p', { class: 'helper', text: '此任务尚未提交文件产物。' }));

  const actions = el('div', { class: 'task-detail-actions' });
  if (onDelegate && mode === 'automatic' && !['failed', 'cancelled'].includes(task.status)) actions.append(button('派发子任务', onDelegate, 'button button-secondary'));
  if (task.status === 'queued' || task.status === 'leased') {
    actions.append(button('在执行面板查看', () => {
      ctx.state.actor = task.assignee;
      onLeave(); ctx.navigate('runner');
    }, 'button button-primary'));
  }
  if (actions.childElementCount) body.append(actions);
  return body;
}

function detail(label, value) {
  return el('div', {}, el('dt', { text: label }), el('dd', {}, value));
}

function relatedTask(ctx, label, id, onSelect) {
  const target = ctx.state.tasks.find(task => task.id === id);
  return el('div', { class: 'task-related' }, el('span', { class: 'helper', text: label }),
    target && onSelect
      ? button(target.title, () => onSelect(id), 'button button-small button-quiet')
      : el('code', { class: 'helper', text: id }));
}
