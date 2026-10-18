import { el, empty } from '../ui.js';
import { getTaskFlows, layoutTaskFlow, coordinationLabel } from '../task-graph-model.js';

export { getTaskFlows } from '../task-graph-model.js';

const SVG_NS = 'http://www.w3.org/2000/svg';
const COLORS = ['indigo', 'teal', 'orange', 'purple'];
const HEADER_HEIGHT = 40;
const STATUS = {
  queued: ['○', '待领取'], leased: ['◉', '执行中'], blocked: ['‖', '等待依赖'],
  completed: ['✓', '已完成'], failed: ['!', '失败'], cancelled: ['×', '已取消'],
};
let graphSequence = 0;

function svg(tag, attributes = {}, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, String(value));
  for (const child of children.flat()) if (child) node.append(child);
  return node;
}

function identity(ctx, id) {
  if (id === '__operator') return { name: '管理员', initial: '管', color: 'indigo', detail: '工作区管理员' };
  const agents = ctx.state.agents || [];
  const index = agents.findIndex(agent => agent.id === id);
  const agent = agents[index];
  if (!agent) return { name: id || '未知身份', initial: '?', color: 'muted', detail: id ? '未注册 / 未载入身份' : '身份未记录' };
  const name = agent.name || agent.id;
  return { name, initial: Array.from(name)[0] || '?', color: COLORS[index % COLORS.length], detail: agent.active === false ? '已停用 Agent' : '执行 Agent' };
}

function avatar(person) {
  return el('span', { class: `task-graph-avatar avatar-${person.color}`, text: person.initial, 'aria-hidden': 'true' });
}

function status(task) {
  const known = Object.hasOwn(STATUS, task.status);
  const [symbol, label] = known ? STATUS[task.status] : ['?', task.status || '未知状态'];
  return el('span', { class: `badge task-graph-status status-${known ? task.status : 'unknown'}` }, el('span', { text: symbol, 'aria-hidden': 'true' }), label);
}

function summaryContent(ctx, task, options) {
  const sender = identity(ctx, task.sender);
  const recipient = identity(ctx, task.reply_to || task.sender);
  const artifacts = Array.isArray(task.result?.artifacts) ? task.result.artifacts.length : 0;
  const attempt = Number.isFinite(Number(task.attempt)) ? Number(task.attempt) : 0;
  const summary = task.result?.summary || task.summary || task.last_error || '尚未提交交付摘要。';
  const preview = (label, text, className) => el('section', { class: 'task-graph-summary-section' }, el('h4', { text: label }), el('p', { class: className, text }));
  return [
    el('button', { type: 'button', class: 'task-graph-summary-close', 'data-action': 'collapse', 'aria-label': '收起任务摘要', onclick: () => options.onSelect?.(null), text: '×' }),
    el('div', { class: 'task-graph-summary-heading' }, el('h3', { text: task.title || '未命名任务' })),
    el('div', { class: 'task-graph-summary-stats' }, status(task), el('span', { text: `尝试 ${attempt}${task.max_attempts ? ` / ${task.max_attempts}` : ''}` }), el('span', { text: `交付 ${artifacts} 个文件` })),
    el('div', { class: 'task-graph-summary-body' },
      preview('任务提示词', task.instructions || '尚未填写任务提示词。', 'task-graph-summary-prompt'),
      preview('交付摘要', summary, 'task-graph-summary-result'),
      el('dl', { class: 'task-graph-summary-people' },
        el('div', {}, el('dt', { text: '发起者' }), el('dd', { text: sender.name })),
        el('div', {}, el('dt', { text: '结果通知' }), el('dd', { text: recipient.name })))),
    el('footer', { class: 'task-graph-summary-footer' },
      el('code', { title: task.id, text: task.id }),
      el('button', { type: 'button', class: 'button button-primary button-small', 'data-action': 'details', onclick: () => options.onDetails?.(task.id), text: '详细内容' })),
  ];
}

function taskNode(ctx, node, options, state) {
  if (node.reference) {
    const source = node.reference;
    const person = identity(ctx, source.assignee);
    const statusText = STATUS[source.status]?.[1] || source.status || '未知状态';
    // 这是其他流程的真实输入引用，点击查看来源详情，不扩展为当前流程的阶段。
    const header = el('button', { type: 'button', class: 'task-graph-reference',
      'data-reference-task-id': source.id, 'data-status': source.status || 'unknown',
      'aria-label': `跨流程依赖：${source.title || '未命名任务'}；来源 Agent：${person.name}；${coordinationLabel(source)}；${statusText}`,
      onclick: () => options.onDetails?.(source.id),
    }, avatar(person), el('span', { class: 'task-graph-name', text: `跨流程 · ${person.name}` }),
    el('span', { class: 'task-graph-state-mark', 'aria-hidden': 'true', text: STATUS[source.status]?.[0] || '?' }));
    const morph = el('div', { class: 'task-graph-morph', 'data-status': source.status || 'unknown' }, header);
    return { shell: el('div', { class: 'task-graph-node-shell is-reference', 'data-node-id': node.id }, morph), morph, header, content: [] };
  }
  if (!node.task) {
    const header = el('div', { class: 'task-graph-missing', tabindex: '0', role: 'img', 'aria-label': `关联任务未载入：${node.id}` },
      avatar({ initial: '?', color: 'muted' }), el('span', { class: 'task-graph-name', text: '关联任务未载入' }));
    const morph = el('div', { class: 'task-graph-morph' }, header);
    return { shell: el('div', { class: 'task-graph-node-shell is-missing', 'data-node-id': node.id }, morph), morph, header, content: [] };
  }
  const task = node.task;
  const person = identity(ctx, task.assignee);
  const sender = identity(ctx, task.sender);
  const recipient = identity(ctx, task.reply_to || task.sender);
  const context = options.highlightIds && !options.highlightIds.has(task.id);
  const attempt = Number.isFinite(Number(task.attempt)) ? Number(task.attempt) : 0;
  const artifacts = Array.isArray(task.result?.artifacts) ? task.result.artifacts.length : 0;
  const statusText = STATUS[task.status]?.[1] || task.status || '未知状态';
  const accessible = `${task.title || '未命名任务'}；${coordinationLabel(task)}；执行 Agent：${person.name}；${statusText}；发起者：${sender.name}；结果通知：${recipient.name}；尝试 ${attempt} 次；产出 ${artifacts} 个文件`;
  const header = el('button', {
    type: 'button', class: `task-graph-node${context ? ' is-context' : ''}`,
    'aria-label': accessible, 'aria-pressed': 'false', 'aria-expanded': 'false',
    'data-task-id': task.id, 'data-status': task.status || 'unknown',
    onclick: () => options.onSelect?.(state.selectedId === task.id ? null : task.id),
  }, avatar(person), el('span', { class: 'task-graph-name', text: person.name }),
  el('span', { class: 'task-graph-state-mark', 'aria-hidden': 'true', text: STATUS[task.status]?.[0] || '?' }));
  const content = summaryContent(ctx, task, options);
  const morph = el('article', { class: `task-graph-morph${context ? ' is-context' : ''}`, 'data-status': task.status || 'unknown' }, header, content);
  const shell = el('div', { class: `task-graph-node-shell${context ? ' is-context' : ''}`, 'data-node-id': node.id }, morph);
  return { shell, morph, header, content, task };
}

function endpointNode(ctx, endpoint) {
  const person = identity(ctx, endpoint.identityId);
  const role = endpoint.kind === 'initiator' ? '任务发起者' : '结果通知接收者';
  const header = el('div', {
    class: `task-graph-endpoint task-graph-${endpoint.kind}`, tabindex: '0', role: 'img',
    'aria-label': `${person.name} · ${person.detail} · ${role} · ${endpoint.taskIds.length} 项任务`,
  }, avatar(person), el('span', { class: 'task-graph-name', text: person.name }));
  const morph = el('div', { class: 'task-graph-morph' }, header);
  return { shell: el('div', { class: `task-graph-node-shell is-${endpoint.kind}`, 'data-node-id': endpoint.id }, morph), morph, header, content: [] };
}

function setBounds(element, bounds) {
  // SVG 属性同步真实外框几何，不写 inline style，遵守 style-src 'self'。
  for (const key of ['x', 'y', 'width', 'height']) element.setAttribute(key, String(bounds[key]));
}

function marker(id, kind) {
  return svg('marker', { id, viewBox: '0 0 8 8', refX: 7, refY: 4, markerWidth: 8, markerHeight: 8, orient: 'auto', markerUnits: 'userSpaceOnUse' }, svg('path', { d: 'M 0 0 L 8 4 L 0 8 Z', class: `task-graph-arrow-${kind}` }));
}

function edgePath(from, to, index, obstacles) {
  const roundedRoute = points => {
    const route = points.filter((point, position) => !position || point[0] !== points[position - 1][0] || point[1] !== points[position - 1][1]);
    let d = `M ${route[0][0]} ${route[0][1]}`;
    for (let position = 1; position < route.length - 1; position++) {
      const previous = route[position - 1];
      const point = route[position];
      const next = route[position + 1];
      const incoming = Math.hypot(point[0] - previous[0], point[1] - previous[1]);
      const outgoing = Math.hypot(next[0] - point[0], next[1] - point[1]);
      const radius = Math.min(8, incoming / 2, outgoing / 2);
      const entry = point.map((value, axis) => value + (previous[axis] - value) * radius / incoming);
      const exit = point.map((value, axis) => value + (next[axis] - value) * radius / outgoing);
      d += ` L ${entry[0]} ${entry[1]} Q ${point[0]} ${point[1]}, ${exit[0]} ${exit[1]}`;
    }
    const last = route[route.length - 1];
    return `${d} L ${last[0]} ${last[1]}`;
  };
  const sx = from.x + from.width + 2;
  const sy = from.y + HEADER_HEIGHT / 2;
  const ty = to.y + HEADER_HEIGHT / 2;
  const forward = to.x > from.x + from.width;
  const tx = forward ? to.x - 5 : to.x + to.width + 5;
  if (forward) {
    const blocked = obstacles.find(node => node.id !== from.id && node.id !== to.id && node.x < tx && node.x + node.width > sx && node.y + node.height > Math.min(sy, ty) && node.y < Math.max(sy, ty));
    if (blocked) {
      // 同列兄弟的箭头起点可能在展开框横幅内：先在原行退出右缘，再向上绕行。
      // 目标若也落在横幅内，则先在框左侧下降，最后沿目标头像行水平接入。
      const routeY = blocked.y - 24 - index % 2 * 12;
      const left = blocked.x - 24;
      const right = blocked.x + blocked.width + 24;
      const sourceTurn = sx + 20 > left && sx + 20 < right ? right : sx + 20;
      const targetTurn = tx - 20 > left && tx - 20 < right ? left : tx - 20;
      return {
        d: roundedRoute([[sx, sy], [sourceTurn, sy], [sourceTurn, routeY], [targetTurn, routeY], [targetTurn, ty], [tx, ty]]),
        labelX: (sourceTurn + targetTurn) / 2, labelY: routeY - 8,
      };
    }
    const middle = (sx + tx) / 2;
    return { d: `M ${sx} ${sy} C ${middle} ${sy}, ${middle} ${ty}, ${tx} ${ty}`, labelX: middle, labelY: (sy + ty) / 2 - 10 };
  }
  // 同列、回向和自引用绕外框右侧；展开后仍接头像所在的上沿行。
  const right = Math.max(sx, tx, ...obstacles.filter(node => node.y < Math.max(sy, ty) && node.y + node.height > Math.min(sy, ty)).map(node => node.x + node.width));
  const bend = right + 38 + index % 3 * 14;
  if (from.id === to.id) {
    const start = from.y + 12;
    const end = from.y + 29;
    return { d: `M ${sx} ${start} C ${bend} ${start}, ${bend} ${end}, ${tx} ${end}`, labelX: bend + 6, labelY: sy - 5, anchor: 'start' };
  }
  // 大贝塞尔即便控制点在框外，也会在回收段切进正文；圆角折线全程走右侧。
  return { d: roundedRoute([[sx, sy], [bend, sy], [bend, ty], [tx, ty]]), labelX: bend + 3, labelY: (sy + ty) / 2 - 8, anchor: 'start' };
}

function edgeRecord(edge, index, prefix) {
  const kind = edge.type === 'notification' ? 'identity' : edge.type;
  const path = svg('path', { class: 'task-graph-connector', 'marker-end': `url(#${prefix}-${kind})` });
  const label = svg('text', { class: 'task-graph-edge-label' }, document.createTextNode(edge.label));
  const group = svg('g', { class: `task-graph-edge task-graph-edge-${kind}`, 'aria-hidden': 'true' }, path, label);
  return { fromId: typeof edge.from === 'string' ? edge.from : edge.from.id, toId: typeof edge.to === 'string' ? edge.to : edge.to.id, kind, edge, index, group, path, label };
}

function allNodes(layout) { return [...layout.nodes, ...layout.initiators, ...layout.recipients]; }

function flowDiagram(ctx, flow, options, index) {
  const state = { selectedId: flow.taskIds.includes(options.selectedId) ? options.selectedId : null, hoveredId: null, focusedId: null };
  const capsuleWidths = new Map();
  const effectiveHover = () => state.hoveredId || state.focusedId;
  const getLayout = () => layoutTaskFlow(flow, { selectedId: state.selectedId, hoveredId: effectiveHover(), capsuleWidths });
  const initial = getLayout();
  const prefix = `task-flow-${++graphSequence}`;
  const canvas = svg('svg', { class: 'task-graph-canvas', width: initial.width, height: initial.height, viewBox: `0 0 ${initial.width} ${initial.height}`, role: 'group', 'aria-label': `${flow.title}，${flow.tasks.length} 项任务的协作关系` });
  canvas.append(svg('defs', {}, marker(`${prefix}-parent`, 'parent'), marker(`${prefix}-dependency`, 'dependency'), marker(`${prefix}-identity`, 'identity')));
  const edges = [...initial.identityEdges, ...flow.edges].map((edge, edgeIndex) => edgeRecord(edge, edgeIndex, prefix));
  for (const edge of edges) canvas.append(edge.group);
  const records = new Map();
  const geometry = new Map(allNodes(initial).map(node => [node.id, { ...node }]));
  let canvasSize = { width: initial.width, height: initial.height };
  let animationFrame = 0;
  let disposed = false;

  function syncState() {
    for (const [id, record] of records) {
      const expanded = Boolean(record.task && id === state.selectedId);
      const hovered = id === effectiveHover();
      for (const element of [record.shell, record.morph]) {
        element.classList.toggle('is-expanded', expanded);
        element.classList.toggle('is-hovered', hovered);
      }
      record.morph.classList.toggle('task-graph-summary', expanded);
      record.header.classList.toggle('is-selected', expanded);
      if (record.task) {
        record.header.setAttribute('aria-expanded', String(expanded));
        record.header.setAttribute('aria-pressed', String(expanded));
        if (expanded) record.morph.setAttribute('data-task-id', id);
        else record.morph.removeAttribute('data-task-id');
      }
      for (const content of record.content) {
        // 收起先停用交互并淡出，尺寸缩回头像后才隐藏布局，正文不会突然换成空白。
        content.hidden = !expanded && geometry.get(id).height <= HEADER_HEIGHT + .5;
        content.inert = !expanded;
        content.setAttribute('aria-hidden', String(!expanded));
      }
    }
  }

  function paint() {
    canvas.setAttribute('width', String(canvasSize.width));
    canvas.setAttribute('height', String(canvasSize.height));
    canvas.setAttribute('viewBox', `0 0 ${canvasSize.width} ${canvasSize.height}`);
    for (const [id, record] of records) {
      const bounds = geometry.get(id);
      setBounds(record.foreignObject, bounds);
      if (id !== state.selectedId && bounds.height <= HEADER_HEIGHT + .5) {
        for (const content of record.content) content.hidden = true;
      }
    }
    const obstacles = [...geometry.values()].filter(node => node.height > HEADER_HEIGHT + 1);
    for (const record of edges) {
      const from = geometry.get(record.fromId);
      const to = geometry.get(record.toId);
      if (!from || !to) continue;
      const path = edgePath(from, to, record.index, obstacles);
      record.path.setAttribute('d', path.d);
      record.label.setAttribute('x', String(path.labelX));
      record.label.setAttribute('y', String(path.labelY));
      record.label.setAttribute('text-anchor', path.anchor || 'middle');
      record.group.classList.toggle('is-active', from.id === state.selectedId || to.id === state.selectedId);
    }
  }

  function stopAnimation() {
    if (animationFrame) cancelAnimationFrame(animationFrame);
    animationFrame = 0;
  }

  function measureCapsule() {
    const id = effectiveHover();
    const record = records.get(id);
    if (!record?.header.isConnected || capsuleWidths.has(id)) return;
    const name = record.header.querySelector('.task-graph-name');
    if (!name) return;
    // Range 测量字形本身，不受当前圆形外框或省略号裁切影响。
    // 40px 已包含头像与两侧基础留白，另加姓名间距和姓名内边距即可。
    // 不计入展开内容框为关闭按钮预留的 padding-right。
    const range = document.createRange();
    range.selectNodeContents(name);
    const headerStyle = getComputedStyle(record.header);
    const nameStyle = getComputedStyle(name);
    const pixels = value => parseFloat(value) || 0;
    const width = HEADER_HEIGHT + range.getBoundingClientRect().width +
      pixels(headerStyle.columnGap) + pixels(nameStyle.paddingLeft) + pixels(nameStyle.paddingRight);
    capsuleWidths.set(id, width);
  }

  function updateLayout(animate = true) {
    if (disposed) return;
    syncState();
    measureCapsule();
    const target = getLayout();
    const targetNodes = new Map(allNodes(target).map(node => [node.id, node]));
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    stopAnimation();
    const finish = () => {
      for (const [id, node] of targetNodes) geometry.set(id, { ...node });
      canvasSize = { width: target.width, height: target.height };
      animationFrame = 0;
      syncState();
      paint();
    };
    if (!animate || reduced || !canvas.isConnected) { finish(); return; }
    const starts = new Map([...geometry].map(([id, node]) => [id, { ...node }]));
    const startSize = { ...canvasSize };
    const differs = ['width', 'height'].some(key => Math.abs(startSize[key] - target[key]) > .01) || [...starts].some(([id, node]) => ['x', 'y', 'width', 'height'].some(key => Math.abs(node[key] - targetNodes.get(id)[key]) > .01));
    if (!differs) { finish(); return; }
    let progress = 0;
    let velocity = 0;
    let previous = performance.now();
    const started = previous;
    const frame = now => {
      // 离开 DOM 后立即结束帧循环，旧图永远不会更新新的选择。
      if (disposed || !canvas.isConnected) { animationFrame = 0; return; }
      const dt = Math.min(.025, Math.max(.001, (now - previous) / 1000));
      previous = now;
      // 轻微回弹也同步外框与箭头，避免视觉弹簧和实际可点击边界分离。
      velocity += (240 * (1 - progress) - 20 * velocity) * dt;
      progress += velocity * dt;
      for (const [id, start] of starts) {
        const node = geometry.get(id);
        const end = targetNodes.get(id);
        for (const key of ['x', 'y', 'width', 'height']) node[key] = start[key] + (end[key] - start[key]) * progress;
        node.width = Math.max(HEADER_HEIGHT, node.width);
        node.height = Math.max(HEADER_HEIGHT, node.height);
      }
      for (const key of ['width', 'height']) canvasSize[key] = startSize[key] + (target[key] - startSize[key]) * progress;
      paint();
      if (now - started > 1200 || (Math.abs(1 - progress) < .001 && Math.abs(velocity) < .015)) { finish(); return; }
      animationFrame = requestAnimationFrame(frame);
    };
    animationFrame = requestAnimationFrame(frame);
  }

  for (const node of allNodes(initial)) {
    const record = node.kind ? endpointNode(ctx, node) : taskNode(ctx, node, options, state);
    record.foreignObject = svg('foreignObject', { x: node.x, y: node.y, width: node.width, height: node.height }, record.shell);
    records.set(node.id, record);
    record.shell.addEventListener('pointerenter', () => { state.hoveredId = node.id; updateLayout(); });
    record.shell.addEventListener('pointerleave', () => { if (state.hoveredId === node.id) { state.hoveredId = null; updateLayout(); } });
    record.shell.addEventListener('focusin', event => {
      // 只把键盘可见焦点当成胶囊状态；鼠标点击后的 focus() 不重启展开动画。
      if (event.target.matches(':focus-visible') && state.focusedId !== node.id) { state.focusedId = node.id; updateLayout(); }
    });
    record.shell.addEventListener('focusout', event => {
      if (!record.shell.contains(event.relatedTarget) && state.focusedId === node.id) { state.focusedId = null; updateLayout(); }
    });
    canvas.append(record.foreignObject);
  }
  syncState();
  paint();
  const isFixed = flow.collaborationMode === 'fixed';
  const notes = [isFixed ? '路线已提前定义，后续阶段等待前一阶段完成。' : '实际派发记录 · 后续协作者由 Agent 决定，新任务产生后这里继续展开。'];
  if (flow.externalIds.length) notes.push(`${flow.externalIds.length} 个跨流程依赖以引用头像显示，点击查看来源详情`);
  if (flow.missingIds.length) notes.push(`${flow.missingIds.length} 个关联任务未载入`);
  if (flow.hasCycle) notes.push('含回向关联，按真实箭头阅读');
  if (initial.compressed) notes.push('深层任务并入末列');
  const section = el('section', { class: 'task-graph-flow task-flow-card', 'data-flow-id': flow.id, 'data-mode': flow.collaborationMode,
    onkeydown: event => { if (event.key === 'Escape' && state.selectedId) { event.preventDefault(); event.stopPropagation(); options.onSelect?.(null); } },
  },
  el('div', { class: 'task-graph-flow-heading' }, el('div', {}, el('span', { class: 'task-graph-flow-number', text: `${isFixed ? '固定执行流程' : '实际流转记录'} ${String(index + 1).padStart(2, '0')}` }), el('h3', { text: flow.title })),
    el('div', { class: 'task-graph-flow-meta' }, el('span', { class: `badge mode-${flow.collaborationMode}`, text: isFixed ? '固定编排' : 'Agent 自动配合' }), el('span', { class: 'task-graph-flow-count', text: `${flow.tasks.length} 项任务` }))),
  notes.length ? el('p', { class: 'task-graph-flow-note', text: notes.join(' · ') }) : null,
  el('div', { class: 'task-graph-viewport', tabindex: '0', role: 'region', 'aria-label': `${flow.title}流程图，可使用方向键滚动` }, canvas));
  return {
    section,
    setSelection(id, config = {}) {
      const selectedId = flow.taskIds.includes(id) ? id : null;
      if (selectedId === state.selectedId || disposed) return;
      state.selectedId = selectedId;
      updateLayout(config.animate ?? options.animateSelection !== false);
    },
    isAnimating: () => Boolean(animationFrame),
    destroy() { disposed = true; stopAnimation(); },
  };
}

/**
 * 构造图形视图。选择更新复用原 DOM；调用方继续管理详情、筛选器与快照刷新。
 * setSelection(taskId | null, { animate })、isAnimating()、destroy() 不修改 ctx.state。
 */
export function renderTaskGraph(ctx, tasks, options = {}) {
  const highlights = options.highlightIds == null ? null : new Set(options.highlightIds);
  const config = { ...options, highlightIds: highlights };
  const flows = getTaskFlows(tasks, ctx.state.tasks || tasks).filter(flow => !options.flowId || flow.id === options.flowId);
  if (!flows.length) {
    const root = empty('没有符合条件的任务流程', '创建任务后，这里会显示执行 Agent、派发关联、执行依赖和结果通知对象。');
    root.setSelection = () => {};
    root.isAnimating = () => false;
    root.destroy = () => {};
    return root;
  }
  const legend = el('div', { class: 'task-graph-legend' },
    el('span', {}, el('i', { class: 'task-graph-legend-line task-graph-legend-parent', 'aria-hidden': 'true' }), '派发子任务'),
    el('span', {}, el('i', { class: 'task-graph-legend-line task-graph-legend-dependency', 'aria-hidden': 'true' }), '执行依赖'),
    el('span', {}, el('i', { class: 'task-graph-legend-line task-graph-legend-identity', 'aria-hidden': 'true' }), '任务发起 / 结果通知'),
    el('small', { text: '派发关联不限制执行顺序；执行依赖完成后，等待任务才可恢复。' }));
  const scenes = flows.map((flow, index) => flowDiagram(ctx, flow, config, index));
  const root = el('div', { class: 'task-graph' }, legend, scenes.map(scene => scene.section));
  root.setSelection = (id, selectionOptions = {}) => { for (const scene of scenes) scene.setSelection(id, selectionOptions); };
  root.isAnimating = () => scenes.some(scene => scene.isAnimating());
  root.destroy = () => { for (const scene of scenes) scene.destroy(); };
  return root;
}
