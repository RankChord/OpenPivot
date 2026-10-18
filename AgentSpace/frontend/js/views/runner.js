import { api } from '../api.js';
import { el, button, field, input, textarea, select, actorField, agentName, pageHeader, panel, empty, when, statusBadge, submit } from '../ui.js';
import { enqueueTask } from './tasks.js';
import { coordinationMode, coordinationLabel } from '../task-graph-model.js';

/**
 * 控制台的执行器模拟：领取返回的 lease_token 仅保存在页面内存，不是 Agent 登录密钥。
 * 所有后续写操作携带任务 ID + lease_token；服务端校验执行身份、有效期和任务状态。
 * 刷新浏览器会丢失本地租约，真实执行器应自己持久化正在执行的任务及租约。
 */
export function renderRunner(ctx) {
  if (!ctx.state.agents.some(a => a.active)) return el('div', {}, pageHeader('AGENT EXECUTION', '执行与交接', '选择任意已启用的身份，验证领取、续租、等待依赖与提交成果。'), empty('先建立一个执行身份', '注册 Agent 后即可模拟它的任务执行流程。', button('安排 Agent', () => ctx.navigate('agents'), 'button button-primary')));
  const actor = ctx.state.actor;
  const held = ctx.state.leases.get(actor);
  const duration = input('lease_seconds', '120', { type: 'number', min: 5, max: 3600, required: true });
  const claimForm = el('form', { class: 'runner-toolbar' }, actorField(ctx, () => ctx.render()), field('租约秒数', duration), el('button', { type: 'submit', class: 'button button-primary', text: '领取下一项任务', disabled: Boolean(held) }));
  claimForm.addEventListener('submit', event => { event.preventDefault(); submit(claimForm, async () => {
    const result = await api.call('task_claim', { lease_seconds: Number(duration.value) }, actor);
    const task = result.task === null ? null : result.task || result;
    if (!task) { ctx.toast('该 Agent 队列当前没有待领取任务'); return; }
    ctx.state.leases.set(actor, task); await ctx.refreshAfterWrite(); ctx.toast(`已领取：${task.title}`);
  }); });
  const agentTasks = ctx.state.tasks.filter(task => task.assignee === actor && ['queued', 'leased', 'blocked'].includes(task.status));
  const queue = el('div', { class: 'runner-queue' }, agentTasks.map(task => el('div', { class: 'queue-row' }, el('div', {}, el('strong', { text: task.title }), el('code', { text: task.id })), statusBadge(task.status))));
  return el('div', {}, pageHeader('AGENT EXECUTION', '执行与交接', '用管理权限模拟 Agent 的 MCP 工具调用，直观看到后端状态机的每一步。'), panel('执行身份与领取', '服务端按队列顺序分配任务；领取成功后才能提交交付。', claimForm), el('div', { class: 'runner-layout' }, panel(held ? `正在执行 · ${agentName(ctx, actor)}` : '当前执行任务', held ? `租约有效期至 ${when(held.lease_until)}` : '领取任务后，这里显示任务说明和交付操作。', held ? execution(ctx, actor, held, duration) : empty('等待领取任务', '选择执行身份并领取下一项任务；真实执行器通过 task_claim 完成同一过程。')), panel('该 Agent 的待办', `${agentTasks.length} 项未完成任务`, agentTasks.length ? queue : empty('队列已清空', '可以创建任务，或让其他 Agent 向它派单。'), [button('派一个任务', () => enqueueTask(ctx, { assignee: actor }), 'button button-small button-quiet')])), el('div', { class: 'notice' }, '此面板执行真实后端请求。它帮助你验证协作流程；模型推理、代码运行和定时轮询由接入的 Agent 执行器负责。页面内存中的租约在刷新标签页后丢失，任务可在租约超时后重新领取。'));
}

function execution(ctx, actor, task, duration) {
  const mode = coordinationMode(task);
  const host = el('div', { class: 'execution-body' }, el('h3', { text: task.title }), el('span', { class: `badge mode-${mode}`, text: coordinationLabel(task) }), el('code', { class: 'helper', text: task.id }), el('pre', { class: 'prose-block', text: task.instructions }), el('div', { class: 'inline-actions' }, button('续租', () => ctx.guard(async () => {
    const result = await api.call('task_heartbeat', { task_id: task.id, lease_token: task.lease_token, lease_seconds: Number(duration.value) }, actor);
    ctx.state.leases.set(actor, result.task || result); await ctx.refreshAfterWrite(); ctx.toast('执行租约已续期');
  }), 'button button-small button-secondary'), mode === 'automatic' ? button('向协作 Agent 派单', () => enqueueTask(ctx, { sender: actor, parent_task_id: task.id, reply_to: actor, collaboration_mode: mode }), 'button button-small button-secondary') : null));
  if (mode === 'fixed') host.append(el('p', { class: 'task-mode-explanation', text: '固定流程按预设阶段交接。提交当前阶段后，下一阶段自动入队。' }));
  const tabs = el('div', { class: 'tab-bar' });
  const content = el('div');
  // 固定路线的依赖在创建流程时确定；执行 Agent 只能交付或报告失败。
  const views = [ ['complete', '提交交付', () => completeForm(ctx, actor, task)],
    ...(mode === 'automatic' ? [['wait', '等待依赖', () => waitForm(ctx, actor, task)]] : []),
    ['fail', '报告失败', () => failForm(ctx, actor, task)] ];
  function activate(key) {
    for (const control of tabs.children) { control.classList.toggle('active', control.dataset.tab === key); control.setAttribute('aria-selected', control.dataset.tab === key ? 'true' : 'false'); }
    content.replaceChildren(views.find(([id]) => id === key)[2]());
  }
  tabs.setAttribute('role', 'tablist');
  for (const [key, label] of views) tabs.append(button(label, () => activate(key), 'tab-button', { 'data-tab': key, role: 'tab' }));
  activate('complete'); host.append(tabs, content); return host;
}

function completeForm(ctx, actor, task) {
  const summary = textarea('summary', '', { required: true, rows: 4, placeholder: '说明完成了什么、测试情况、交付路径，以及下一位 Agent 需要做什么。' });
  const path = input('path', '', { placeholder: '例如 shared/api/openapi.md；可留空' });
  const version = input('expected_version', '0', { type: 'number', min: 0, required: true });
  const content = textarea('content', '', { rows: 7, placeholder: '可随任务提交一个 UTF-8 文本产物。' });
  const form = el('form', { class: 'stack-form' }, field('交付摘要', summary), el('div', { class: 'form-grid' }, field('可选产物路径', path), field('预期当前版本', version, '新文件填 0；更新文件填当前版本。')), field('文本产物内容', content), el('p', { class: 'helper', text: '成果写入、任务完成和结果通知会在同一事务中提交；文件冲突会保留任务，修正版本后可重试。此面板演示单个文本产物；真实执行器可通过 task_complete.outputs 同时提交最多 16 个产物。' }), el('div', { class: 'form-actions' }, el('button', { type: 'submit', class: 'button button-primary', text: '提交交付并通知' })));
  form.addEventListener('submit', event => { event.preventDefault(); submit(form, async () => {
    if (content.value && !path.value.trim()) throw new Error('填写文本产物时，也需要填写产物路径');
    const outputs = path.value.trim() ? [{ path: path.value.trim(), content: content.value, expected_version: Number(version.value) }] : [];
    await api.call('task_complete', { task_id: task.id, lease_token: task.lease_token, summary: summary.value.trim(), outputs }, actor);
    ctx.state.leases.delete(actor); await ctx.refreshAfterWrite(); ctx.toast('交付已提交，结果通知已进入目标收件箱');
  }); }); return form;
}

function waitForm(ctx, actor, task) {
  const dependencies = ctx.state.tasks.filter(item => item.id !== task.id && !['failed', 'cancelled'].includes(item.status));
  const dependency = select('dependency', [{ value: '', label: '选择要等待的任务' }, ...dependencies.map(item => ({ value: item.id, label: `${item.title} · ${agentName(ctx, item.assignee)} · ${item.id.slice(0, 8)}` }))], '', { required: true });
  const form = el('form', { class: 'stack-form' }, field('依赖任务', dependency), el('p', { class: 'helper', text: '依赖完成后，本任务会重新入队。需要再次领取后继续执行；等待资料不会消耗失败重试次数。' }), el('div', { class: 'form-actions' }, el('button', { type: 'submit', class: 'button button-primary', text: '登记依赖并释放租约', disabled: !dependencies.length })));
  form.addEventListener('submit', event => { event.preventDefault(); submit(form, async () => {
    await api.call('task_wait_on', { task_id: task.id, lease_token: task.lease_token, dependency_task_id: dependency.value }, actor);
    ctx.state.leases.delete(actor); await ctx.refreshAfterWrite(); ctx.toast('依赖已登记，任务等待协作交付');
  }); }); return form;
}

function failForm(ctx, actor, task) {
  const reason = textarea('reason', '', { required: true, placeholder: '记录失败原因与已尝试的处理方式。' });
  const retryable = input('retryable', '', { type: 'checkbox', checked: true });
  const form = el('form', { class: 'stack-form' }, field('失败原因', reason), el('label', { class: 'checkbox-field' }, retryable, '允许重试（达到重试上限后仍会最终失败）'), el('div', { class: 'form-actions' }, el('button', { type: 'submit', class: 'button button-danger', text: '提交失败状态' })));
  form.addEventListener('submit', event => { event.preventDefault(); submit(form, async () => {
    await api.call('task_fail', { task_id: task.id, lease_token: task.lease_token, reason: reason.value.trim(), retryable: retryable.checked }, actor);
    ctx.state.leases.delete(actor); await ctx.refreshAfterWrite(); ctx.toast(retryable.checked ? '失败已记录，服务端已按重试次数处理' : '任务已标记为最终失败');
  }); }); return form;
}
