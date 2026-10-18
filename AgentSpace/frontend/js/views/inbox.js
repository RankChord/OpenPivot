import { api, operationKey } from '../api.js';
import { el, button, field, input, textarea, select, agentItems, agentName, pageHeader, panel, empty, when, modal, submit } from '../ui.js';
import { showTask } from './tasks.js';

/** 收件箱 poll 只读取未确认消息；ack 是显式确认，不因页面打开而自动移除通知。 */
export function renderInbox(ctx) {
  const identity = select('inbox_actor', [{ value: '', label: '管理员收件箱' }, ...agentItems(ctx)], ctx.state.inboxActor || '');
  const list = el('div', {}, el('p', { class: 'loading', text: '读取未确认消息…' }));
  let messages = [];
  let sequence = 0;
  async function reload() {
    const requestId = ++sequence;
    ctx.state.inboxActor = identity.value;
    try {
      const result = await api.call('inbox_poll', { limit: 100 }, identity.value || null);
      if (requestId !== sequence) return;
      messages = result.messages;
      list.replaceChildren(messages.length ? el('div', { class: 'message-list' }, messages.map(message => messageCard(ctx, message, identity.value, reload))) : empty('收件箱已清空', '任务完成、失败与其他 Agent 的协作信息会出现在这里。'));
    } catch (error) { if (requestId === sequence) list.replaceChildren(el('p', { class: 'notice notice-error', role: 'alert', text: error.message })); }
  }
  identity.addEventListener('change', reload);
  const ackAll = button('确认本页消息', () => ctx.guard(async () => {
    if (!messages.length) { ctx.toast('当前没有未确认消息'); return; }
    await api.call('inbox_ack', { message_ids: messages.map(message => message.id) }, identity.value || null);
    await reload(); await ctx.refreshAfterWrite(false); ctx.toast('消息已确认');
  }), 'button button-small button-secondary');
  reload();
  return el('div', {}, pageHeader('AGENT COMMUNICATION', '协作收件箱', '消息补齐任务之外的沟通，让交付与下一步要求抵达正确的执行身份。', [button('＋ 发送消息', () => sendMessage(ctx, identity.value, reload), 'button button-primary')]), panel('未确认消息', '消息会一直保留到执行身份显式确认，便于执行器断线后恢复。', el('div', {}, el('div', { class: 'filter-bar' }, identity, button('读取队列', reload, 'button button-secondary')), list), [ackAll]));
}

function messageCard(ctx, message, actor, reload) {
  const labels = { message: '协作消息', task_completed: '交付完成', task_failed: '任务失败', task_cancelled: '任务已撤回' };
  const task = ctx.state.tasks.find(item => item.id === message.task_id);
  const artifacts = message.payload?.artifacts || [];
  const badge = message.kind === 'task_failed' ? 'status-failed' : message.kind === 'task_cancelled' ? 'status-cancelled' : 'badge-info';
  return el('article', { class: 'message-card' }, el('div', { class: 'message-heading' }, el('div', {}, el('span', { class: `badge ${badge}`, text: labels[message.kind] || message.kind }), el('strong', { text: `来自 ${agentName(ctx, message.sender)}` })), el('time', { text: when(message.created_at) })), el('p', { class: 'message-body', text: message.body }), artifacts.length ? el('div', { class: 'artifact-list' }, artifacts.map(artifact => button(`${artifact.path} · v${artifact.version}`, () => { ctx.state.fileActor = ''; ctx.state.filePrefix = artifact.path.slice(0, artifact.path.lastIndexOf('/') + 1); ctx.state.filePath = artifact.path; ctx.state.fileVersion = artifact.version; ctx.navigate('files'); }, 'artifact-link'))) : null, el('div', { class: 'message-actions' }, task ? button(`关联任务：${task.title}`, () => showTask(ctx, task), 'button button-small button-quiet') : message.task_id ? el('code', { class: 'helper', text: message.task_id }) : el('span'), button('确认已读', () => ctx.guard(async () => { await api.call('inbox_ack', { message_ids: [message.id] }, actor || null); await reload(); await ctx.refreshAfterWrite(false); ctx.toast('消息已确认'); }), 'button button-small button-secondary')));
}

export function sendMessage(ctx, initialActor = '', onSent = null) {
  const form = el('form', { class: 'modal-body' });
  const sender = select('sender', [{ value: '', label: '管理员 / 用户' }, ...agentItems(ctx)], initialActor);
  const recipient = select('recipient', [{ value: '__operator', label: '管理员收件箱' }, ...agentItems(ctx)], ctx.state.actor || '__operator', { required: true });
  const body = textarea('body', '', { required: true, rows: 6, maxLength: 16000, placeholder: '说明需要什么资料、资料所在路径或下一步工作。' });
  const task = input('task_id', '', { placeholder: '可选，关联已有任务 ID' });
  const key = operationKey();
  form.append(el('div', { class: 'form-grid' }, field('发送身份', sender), field('发送给', recipient)), field('消息内容', body), field('关联任务 ID', task));
  const dialog = modal('发送协作消息', form);
  form.append(el('div', { class: 'form-actions' }, button('取消', dialog.close), el('button', { type: 'submit', class: 'button button-primary', text: '发送到收件箱' })));
  form.addEventListener('submit', event => { event.preventDefault(); submit(form, async () => {
    const args = { to: recipient.value, body: body.value.trim(), idempotency_key: key }; if (task.value.trim()) args.task_id = task.value.trim();
    await api.call('message_send', args, sender.value || null); dialog.close(); await onSent?.(); await ctx.refreshAfterWrite(false); ctx.toast('消息已送达目标队列');
  }); });
}
