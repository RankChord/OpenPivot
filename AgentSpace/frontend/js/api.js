/**
 * HTTP 边界：所有管理请求同源，并显式携带管理令牌。
 * UI 不直接操作数据库；后端负责权限、状态机、幂等和文件版本校验。
 * 真实 Agent 仍然通过 /mcp 调用工具，此 REST 适配层只供管理员演示和管理。
 */
let adminToken = sessionStorage.getItem('agentspace.adminToken') || '';
export function setToken(token) { adminToken = token.trim(); sessionStorage.setItem('agentspace.adminToken', adminToken); }
export function clearToken() { adminToken = ''; sessionStorage.removeItem('agentspace.adminToken'); }
export function hasToken() { return Boolean(adminToken); }

async function request(path, options = {}) {
  const response = await fetch(path, { ...options, headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${adminToken}`, ...options.headers }, cache: 'no-store' });
  let data;
  try { data = await response.json(); } catch { throw new Error(`服务返回了无效的 JSON（HTTP ${response.status}）`); }
  if (!response.ok) throw new Error(data.error?.message || data.error || data.message || `请求失败：HTTP ${response.status}`);
  return data;
}

export const api = {
  session: () => request('/api/session'),
  snapshot: () => request('/api/snapshot'),
  call: (tool, args = {}, actor = null) => request('/api/call', { method: 'POST', body: JSON.stringify({ tool, arguments: args, actor }) }),
  createAgent: values => request('/api/agents', { method: 'POST', body: JSON.stringify(values) }),
  updateAgent: (id, values) => request(`/api/agents/${encodeURIComponent(id)}`, { method: 'PATCH', body: JSON.stringify(values) }),
  rotateToken: id => request(`/api/agents/${encodeURIComponent(id)}/token`, { method: 'POST', body: '{}' }),
};

/** 每次用户发起新的逻辑操作生成一次 key；重试同一个调用应复用该 key。 */
export function operationKey() { return crypto.randomUUID(); }
