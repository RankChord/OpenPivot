/**
 * DOM 工具：所有服务端数据经 textContent 写入，避免文件、任务或消息里的 HTML 被执行。
 * 固定 UI 结构也通过 createElement 创建，便于在主项目中按需替换成组件框架。
 */
export function el(tag, options = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(options)) {
    if (value == null) continue;
    if (key === 'class') node.className = value;
    else if (key === 'text') node.textContent = String(value);
    else if (key.startsWith('on')) node.addEventListener(key.slice(2).toLowerCase(), value);
    else if (key in node && key !== 'role') node[key] = value;
    else node.setAttribute(key, String(value));
  }
  for (const child of children.flat(Infinity)) {
    if (child != null) node.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return node;
}

export function button(label, onClick, className = 'button button-secondary', extra = {}) {
  return el('button', { type: 'button', class: className, text: label, onclick: onClick, ...extra });
}

export function field(label, input, help = '') {
  return el('label', { class: 'field' }, el('span', { text: label }), input, help ? el('small', { class: 'helper', text: help }) : null);
}

export function input(name, value = '', options = {}) {
  return el('input', { name, value, ...options });
}

export function textarea(name, value = '', options = {}) {
  return el('textarea', { name, value, rows: 4, ...options });
}

export function select(name, items, value = '', options = {}) {
  const node = el('select', { name, ...options }, items.map(item => el('option', { value: item.value, text: item.label })));
  node.value = value;
  return node;
}

export function pageHeader(eyebrow, title, description, actions = []) {
  return el('div', { class: 'page-heading' }, el('div', {}, el('span', { class: 'eyebrow', text: eyebrow }), el('h1', { text: title }), el('p', { text: description })), el('div', { class: 'heading-actions' }, actions));
}

export function panel(title, subtitle, content, actions = []) {
  return el('section', { class: 'panel' }, el('div', { class: 'panel-heading' }, el('div', {}, el('h2', { text: title }), subtitle ? el('p', { text: subtitle }) : null), el('div', { class: 'inline-actions' }, actions)), content);
}

export function empty(title, description, action = null) {
  return el('div', { class: 'empty-state' }, el('span', { class: 'empty-symbol', text: '◇' }), el('h3', { text: title }), el('p', { text: description }), action);
}

const statuses = { queued: '待领取', leased: '执行中', blocked: '等待依赖', completed: '已完成', failed: '失败', cancelled: '已取消' };
export function statusBadge(status) {
  return el('span', { class: `badge status-${status}`, text: statuses[status] || status });
}

export function when(timestamp) {
  if (!timestamp) return '—';
  return new Intl.DateTimeFormat('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false }).format(new Date(timestamp * 1000));
}

export function bytes(size) {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

export function agentName(ctx, id) {
  if (id === '__operator') return '管理员';
  return ctx.state.agents.find(agent => agent.id === id)?.name || id || '—';
}

export function agentItems(ctx, includeDisabled = false) {
  return ctx.state.agents.filter(agent => includeDisabled || agent.active !== false).map(agent => ({ value: agent.id, label: `${agent.name} · ${agent.id}` }));
}

/** 该选择只影响控制台模拟的身份，不修改真实 Agent 的服务端身份。 */
export function actorField(ctx, onChange = null, label = '执行身份') {
  const control = select('actor', agentItems(ctx), ctx.state.actor, { required: true, onchange: event => { ctx.state.actor = event.target.value; onChange?.(event.target.value); } });
  return field(label, control);
}

export function formError(form, message) {
  let node = form.querySelector('.form-error');
  if (!node) { node = el('p', { class: 'form-error', role: 'alert' }); form.append(node); }
  node.textContent = message || '';
}

/** 异步按钮不吞错误：留在当前表单显示，调用方可以修正后重新提交。 */
export async function submit(form, action) {
  const controls = [...form.querySelectorAll('button[type="submit"]')];
  controls.forEach(control => { control.disabled = true; });
  formError(form, '');
  try { await action(); } catch (error) { formError(form, error.message); }
  finally { controls.forEach(control => { control.disabled = false; }); }
}

export function modal(title, body, wide = false) {
  const dialog = el('dialog', { class: `modal${wide ? ' modal-wide' : ''}` });
  const close = () => { dialog.close(); dialog.remove(); };
  dialog.append(el('div', { class: 'modal-header' }, el('h2', { text: title }), button('×', close, 'icon-button', { 'aria-label': '关闭' })), body);
  dialog.addEventListener('cancel', close);
  dialog.addEventListener('click', event => { if (event.target === dialog) close(); });
  document.body.append(dialog);
  dialog.showModal();
  return { dialog, close };
}

export async function copy(ctx, text) {
  await navigator.clipboard.writeText(text);
  ctx.toast('已复制到剪贴板');
}
