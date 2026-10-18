import { api, setToken, clearToken, hasToken } from './api.js';
import { el, empty } from './ui.js';
import { renderOverview } from './views/overview.js';
import { renderAgents } from './views/agents.js';
import { renderTasks } from './views/tasks.js';
import { renderFiles } from './views/files.js';
import { renderInbox } from './views/inbox.js';
import { renderRunner } from './views/runner.js';
import { renderConnection } from './views/connection.js';

/**
 * 应用协调层只负责页面状态和刷新，各视图拥有自己的表单与工具调用。
 * 不启动自动整页刷新，避免清除正在输入的任务或成果。操作成功或用户点击刷新时读取 snapshot。
 * Agent 密钥不会存到 state；唯一持久化信息是 sessionStorage 中的管理员登录令牌。
 */
const routes = {
  overview: { label: '工作区总览', icon: '▦', render: renderOverview },
  agents: { label: 'Agent 团队', icon: '◉', render: renderAgents },
  tasks: { label: '任务与交付', icon: '☷', render: renderTasks },
  files: { label: '文件与记忆', icon: '▤', render: renderFiles },
  inbox: { label: '协作收件箱', icon: '▣', render: renderInbox },
  runner: { label: '执行面板', icon: '▷', render: renderRunner },
  connection: { label: 'MCP 连接', icon: '⌁', render: renderConnection },
};
const state = { agents: [], tasks: [], files: [], stats: {}, actor: '', leases: new Map(), route: routeFromHash() };
const main = document.querySelector('#main');
const status = document.querySelector('#connection-status');
const loginDialog = document.querySelector('#login-dialog');
const refreshButton = document.querySelector('#refresh');
let connected = false;
let snapshotSequence = 0;
let viewDisposers = [];

const ctx = {
  state,
  navigate(route) { if (location.hash === `#${route}`) { state.route = route; render(); } else location.hash = route; },
  render,
  refresh,
  // 视图自行登记计时器/订阅清理；路由切换或整页重绘时统一释放。
  onDispose(callback) { viewDisposers.push(callback); },
  async refreshAfterWrite(redraw = true) {
    // 写操作成功后，刷新失败不能被误报为写入失败，也不能让已经关闭的表单吞掉错误。
    try { await refresh(redraw); }
    catch (error) { if (redraw) render(); toast(`操作已完成，但数据刷新失败：${error.message}`, true); }
  },
  toast,
  async guard(action) { try { return await action(); } catch (error) { toast(error.message, true); return undefined; } },
};

function routeFromHash() { const name = location.hash.slice(1); return routesForHash(name); }
function routesForHash(name) { return ['overview', 'agents', 'tasks', 'files', 'inbox', 'runner', 'connection'].includes(name) ? name : 'overview'; }

function render() {
  for (const dispose of viewDisposers) dispose();
  viewDisposers = [];
  document.querySelector('#page-breadcrumb').textContent = routes[state.route].label;
  document.querySelector('#navigation').replaceChildren(...Object.entries(routes).map(([route, item]) => navItem(route, item)));
  main.replaceChildren(connected ? routes[state.route].render(ctx) : empty('工作区等待连接', '输入管理令牌开始安排 Agent 与任务。'));
}

function navItem(route, item) {
  return el('button', { type: 'button', class: `nav-item${state.route === route ? ' active' : ''}`, 'aria-current': state.route === route ? 'page' : 'false', onclick: () => ctx.navigate(route) }, el('span', { class: 'nav-icon', text: item.icon, 'aria-hidden': true }), el('span', { text: item.label }), route === 'inbox' && state.stats.unread_messages ? el('span', { class: 'nav-count', text: state.stats.unread_messages }) : null);
}

async function refresh(redraw = true) {
  const sequence = ++snapshotSequence;
  refreshButton.disabled = true;
  try {
    const snapshot = await api.snapshot();
    if (sequence !== snapshotSequence) return;
    state.agents = snapshot.agents; state.tasks = snapshot.tasks; state.stats = snapshot.stats;
    if (!state.agents.some(agent => agent.id === state.actor && agent.active)) state.actor = state.agents.find(agent => agent.active)?.id || '';
    // 本地租约已过期或任务已被后端结束时，释放按钮状态，允许重新领取。
    for (const [actor, task] of state.leases) {
      const current = state.tasks.find(item => item.id === task.id);
      // 另一标签页/执行器可能已经重新领取同一任务；不能给旧 token 套上新租约的截止时间。
      if (!current || current.status !== 'leased' || !task.lease_until || task.lease_until <= Date.now() / 1000 || current.attempt !== task.attempt || !current.lease_until || current.lease_until <= Date.now() / 1000) state.leases.delete(actor);
      else state.leases.set(actor, { ...current, lease_token: task.lease_token });
    }
    connected = true; status.textContent = '工作区已连接'; status.classList.add('connected');
    if (!redraw) document.querySelector('#navigation').replaceChildren(...Object.entries(routes).map(([route, item]) => navItem(route, item)));
    if (redraw) render();
  } catch (error) {
    status.textContent = '连接失败'; status.classList.remove('connected');
    throw error;
  } finally { refreshButton.disabled = false; }
}

function toast(message, isError = false) {
  const node = el('div', { class: `toast${isError ? ' toast-error' : ''}`, text: message });
  document.querySelector('#toast-region').append(node); setTimeout(() => node.remove(), isError ? 10000 : 5000);
}

function showLogin() { if (!loginDialog.open) loginDialog.showModal(); }
loginDialog.addEventListener('cancel', event => event.preventDefault());
document.querySelector('#login-form').addEventListener('submit', async event => {
  event.preventDefault(); const form = event.currentTarget; const control = form.querySelector('button'); control.disabled = true;
  document.querySelector('#login-error').textContent = '';
  setToken(document.querySelector('#admin-token').value);
  try { await refresh(); loginDialog.close(); }
  catch (error) { clearToken(); document.querySelector('#login-error').textContent = error.message; }
  finally { control.disabled = false; }
});
refreshButton.addEventListener('click', () => ctx.guard(async () => { await refresh(); toast('工作区数据已刷新'); }));
document.querySelector('#logout').addEventListener('click', () => {
  clearToken(); connected = false; state.leases.clear(); status.textContent = '尚未连接'; status.classList.remove('connected'); render(); showLogin();
});
window.addEventListener('hashchange', () => { state.route = routeFromHash(); render(); });
render();
if (hasToken()) refresh().catch(error => { toast(error.message, true); showLogin(); }); else showLogin();
