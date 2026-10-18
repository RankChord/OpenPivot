/**
 * 任务图的纯数据层，不依赖 DOM，也不修改传入的 snapshot。
 * parent_task_id 是派发关联；depends_on 才表示执行依赖，二者不能互相推导。
 */
const NODE_SIZE = 40;
// 无 DOM 的调用方可省略测量值；浏览器传入按实际字体测量的姓名胶囊宽度。
const CAPSULE_WIDTH = 176;
const MAX_CAPSULE_WIDTH = 260;
const EXPANDED_WIDTH = 320;
const EXPANDED_HEIGHT = 360;
const COLUMN_GAP = 112;
const ROW_GAP = 64;
const MAX_RANK = 5;

/** 模式属于整个流程；旧导入快照未携带字段时保留原来的派发行为。 */
export function coordinationMode(task) { return task?.collaboration_mode === 'fixed' ? 'fixed' : 'automatic'; }
export function coordinationLabel(task) { return coordinationMode(task) === 'fixed' ? '固定编排' : 'Agent 自动配合'; }

function referenceIds(value) {
  // 当前服务端返回单个 ID；兼容数组使部分导入数据也能明确展示。
  return [...new Set((Array.isArray(value) ? value : [value]).filter(id => typeof id === 'string' && id.length))];
}

function taskOrder(left, right) {
  return (Number(left.created_at) || 0) - (Number(right.created_at) || 0) || left.id.localeCompare(right.id);
}

function relations(tasks) {
  return tasks.flatMap(task => [
    ...referenceIds(task.parent_task_id).map(id => ({ from: id, to: task.id, type: 'parent', label: '派发子任务' })),
    ...referenceIds(task.depends_on).map(id => ({ from: id, to: task.id, type: 'dependency', label: '执行依赖' })),
  ]);
}

/**
 * 返回按最近更新排序的连通流程：
 * { id, rootId, title, tasks, taskIds, edges, missingIds, hasCycle }。
 * id 使用流程中最早的根任务 ID；缺失引用仍参与连通分组，避免拆散共同上级的任务。
 * hasCycle 指两类箭头合并后的回向关联，不代表服务端存在执行依赖死锁。
 */
export function getTaskFlows(input = [], referenceTasks = input) {
  const taskMap = new Map();
  for (const task of Array.isArray(input) ? input : []) {
    if (task && typeof task.id === 'string' && task.id && !taskMap.has(task.id)) taskMap.set(task.id, task);
  }
  // 筛选只隐藏其他流程，不把已知依赖误写成缺失任务；参考快照不加入成员。
  const referenceMap = new Map((Array.isArray(referenceTasks) ? referenceTasks : []).filter(task => task?.id).map(task => [task.id, task]));
  for (const [id, task] of taskMap) referenceMap.set(id, task);
  const tasks = [...taskMap.values()].sort(taskOrder);
  const edges = relations(tasks);
  const neighbors = new Map(tasks.map(task => [task.id, new Set()]));
  // 流程身份只用于分组，不绘制虚构的执行边；真实箭头仍来自派发和依赖。
  const workflows = new Map();
  for (const task of tasks) {
    if (!task.workflow_id) continue;
    const first = workflows.get(task.workflow_id);
    if (first) { neighbors.get(first).add(task.id); neighbors.get(task.id).add(first); }
    else workflows.set(task.workflow_id, task.id);
  }
  for (const edge of edges) {
    const source = referenceMap.get(edge.from);
    const target = taskMap.get(edge.to);
    // 跨流程依赖是实际输入关系，但不能改变流程身份或协作模式。
    if (source?.workflow_id && target?.workflow_id && source.workflow_id !== target.workflow_id) continue;
    if (!neighbors.has(edge.from)) neighbors.set(edge.from, new Set());
    neighbors.get(edge.from).add(edge.to);
    neighbors.get(edge.to).add(edge.from);
  }
  const seen = new Set();
  const flows = [];
  for (const task of tasks) {
    if (seen.has(task.id)) continue;
    const ids = new Set([task.id]);
    const queue = [task.id];
    seen.add(task.id);
    for (let cursor = 0; cursor < queue.length; cursor++) {
      for (const next of neighbors.get(queue[cursor]) || []) {
        const member = taskMap.get(next);
        // 缺失引用可能被多个流程共用，也不能通过这个占位节点合并流程身份。
        if (member && (task.workflow_id || member.workflow_id) && member.workflow_id !== task.workflow_id) continue;
        if (ids.has(next) || member && seen.has(next)) continue;
        if (member) seen.add(next);
        ids.add(next); queue.push(next);
      }
    }
    const members = tasks.filter(item => ids.has(item.id));
    const mode = coordinationMode(members[0]);
    const root = members.find(item => item.id === item.workflow_id) || members.find(item =>
      !referenceIds(item.parent_task_id).some(id => taskMap.has(id)) &&
      (mode !== 'fixed' || !referenceIds(item.depends_on).some(id => taskMap.has(id)))) || members[0];
    const inputEdges = edges.filter(edge => ids.has(edge.to));
    const externalIds = [...new Set(inputEdges.filter(edge => !ids.has(edge.from) && referenceMap.has(edge.from)).map(edge => edge.from))].sort();
    const flow = {
      id: root.workflow_id || root.id,
      rootId: root.id,
      title: root.workflow_title || root.title || '未命名任务',
      collaborationMode: mode,
      workflowId: root.workflow_id || null,
      tasks: members,
      taskIds: members.map(item => item.id),
      edges: inputEdges,
      missingIds: [...ids].filter(id => !taskMap.has(id)).sort(),
      externalIds,
      externalTasks: externalIds.map(id => referenceMap.get(id)),
    };
    flow.hasCycle = rankNodes([...flow.externalIds, ...flow.missingIds, ...flow.taskIds], flow.edges).hasCycle;
    flows.push(flow);
  }
  return flows.sort((left, right) => {
    const updated = flow => flow.tasks.reduce((latest, task) => Math.max(latest, Number(task.updated_at) || Number(task.created_at) || 0), 0);
    return updated(right) - updated(left) || left.id.localeCompare(right.id);
  });
}

function rankNodes(ids, edges) {
  const rank = new Map(ids.map(id => [id, 0]));
  const incoming = new Map(ids.map(id => [id, 0]));
  const outgoing = new Map(ids.map(id => [id, []]));
  for (const edge of edges) {
    outgoing.get(edge.from)?.push(edge.to);
    incoming.set(edge.to, (incoming.get(edge.to) || 0) + 1);
  }
  const ready = ids.filter(id => incoming.get(id) === 0);
  const visited = new Set();
  for (let cursor = 0; cursor < ready.length; cursor++) {
    const id = ready[cursor];
    visited.add(id);
    for (const next of outgoing.get(id) || []) {
      rank.set(next, Math.max(rank.get(next) || 0, (rank.get(id) || 0) + 1));
      incoming.set(next, incoming.get(next) - 1);
      if (incoming.get(next) === 0) ready.push(next);
    }
  }
  // 合并图常见“上级派发给子任务，同时等待子任务”。只排版回向箭头，不误判执行顺序。
  // Kahn 队列只访问一次；环和环后的节点落在同一末列，保证循环引用也不会挂起。
  const resolvedRank = [...visited].reduce((highest, id) => Math.max(highest, rank.get(id)), 0);
  const unresolvedRank = Math.min(MAX_RANK, resolvedRank + (visited.size ? 1 : 0));
  for (const id of ids) rank.set(id, Math.min(MAX_RANK, visited.has(id) ? rank.get(id) : unresolvedRank));
  return { rank, hasCycle: visited.size !== ids.length };
}

/**
 * 计算三态节点的分层布局，最多六列；长链末端由真实箭头保留关联。
 * 发起身份取各派发根任务，通知身份按 reply_to 分组。
 * 胶囊和展开正文都是原节点的真实边界，不再另设悬挂摘要。
 * 列宽只向后让位，行带只向下让位，节点自己的左上角不受自身展开影响。
 * 返回 { nodes, initiators, recipients, identityEdges, width, height, compressed }。
 */
export function layoutTaskFlow(flow, options = {}) {
  const capsuleWidth = id => {
    const measured = options.capsuleWidths?.get(id);
    return Number.isFinite(measured)
      ? Math.ceil(Math.min(MAX_CAPSULE_WIDTH, Math.max(NODE_SIZE, measured)))
      : CAPSULE_WIDTH;
  };
  // 单任务三方头像在窄屏也完整可见；多层图继续为关系文字保留间距。
  const columnGap = flow.taskIds.length === 1 && flow.missingIds.length === 0 && !flow.externalIds?.length ? 80 : COLUMN_GAP;
  const ids = [...(flow.externalIds || []), ...flow.missingIds, ...flow.taskIds];
  const { rank } = rankNodes(ids, flow.edges);
  const taskMap = new Map(flow.tasks.map(task => [task.id, task]));
  const externalMap = new Map((flow.externalTasks || []).map(task => [task.id, task]));
  const rows = new Map();
  const nodes = ids.map(id => {
    const column = rank.get(id) || 0;
    const row = rows.get(column) || 0;
    rows.set(column, row + 1);
    const task = taskMap.get(id) || null;
    const expanded = Boolean(task && id === options.selectedId);
    const hovered = id === options.hoveredId;
    return {
      id, task, reference: externalMap.get(id) || null, column, row, x: 0, y: 0,
      width: expanded ? EXPANDED_WIDTH : hovered ? capsuleWidth(id) : NODE_SIZE,
      height: expanded ? EXPANDED_HEIGHT : NODE_SIZE,
      headerHeight: NODE_SIZE, expanded, hovered,
    };
  });
  const lastRank = [...rank.values()].reduce((highest, value) => Math.max(highest, value), 0);
  // 固定阶段提前全部入库，parent 均为空；执行依赖也算输入，只连接第一阶段。
  const roots = flow.tasks.filter(task => !referenceIds(task.parent_task_id).some(id => taskMap.has(id)) &&
    (flow.collaborationMode !== 'fixed' || !referenceIds(task.depends_on).some(id => taskMap.has(id))));
  const initiatingTasks = roots.length ? roots : flow.tasks.filter(task => task.id === flow.rootId);
  const initiatorGroups = new Map();
  const recipientGroups = new Map();
  for (const task of initiatingTasks) {
    const id = task.sender || '';
    if (!initiatorGroups.has(id)) initiatorGroups.set(id, []);
    initiatorGroups.get(id).push(task.id);
  }
  for (const task of flow.tasks) {
    const id = task.reply_to || task.sender || '';
    if (!recipientGroups.has(id)) recipientGroups.set(id, []);
    recipientGroups.get(id).push(task.id);
  }
  const rowHeights = [];
  const columnWidths = Array(lastRank + 1).fill(NODE_SIZE);
  for (const node of nodes) {
    rowHeights[node.row] = Math.max(rowHeights[node.row] || NODE_SIZE, node.height);
    columnWidths[node.column] = Math.max(columnWidths[node.column], node.width);
  }
  const rowPositions = [];
  let rowTop = 54;
  for (let row = 0; row < rowHeights.length; row++) {
    rowPositions[row] = rowTop;
    rowTop += (rowHeights[row] || NODE_SIZE) + ROW_GAP;
  }
  for (const node of nodes) node.y = rowPositions[node.row];
  const nodeMap = new Map(nodes.map(node => [node.id, node]));
  const endpoints = (groups, kind) => {
    const result = [...groups].map(([identityId, taskIds], index) => {
      const id = `${kind}:${index}`;
      const related = taskIds.map(taskId => nodeMap.get(taskId)).filter(Boolean);
      // 身份端点跟随所关联任务的头像行，展开正文不会把左右头像拖到卡片中线。
      const y = related.length ? related.reduce((sum, node) => sum + node.y, 0) / related.length : 54;
      return { id, identityId, taskIds, kind, x: 0, y, width: options.hoveredId === id ? capsuleWidth(id) : NODE_SIZE, height: NODE_SIZE, headerHeight: NODE_SIZE, hovered: options.hoveredId === id };
    });
    let previousBottom = -ROW_GAP;
    for (const endpoint of [...result].sort((left, right) => left.y - right.y || left.id.localeCompare(right.id))) {
      endpoint.y = Math.max(endpoint.y, previousBottom + ROW_GAP);
      previousBottom = endpoint.y + endpoint.height;
    }
    return result;
  };
  const initiators = endpoints(initiatorGroups, 'initiator');
  const recipients = endpoints(recipientGroups, 'recipient');
  const initiatorWidth = Math.max(NODE_SIZE, ...initiators.map(endpoint => endpoint.width));
  const recipientWidth = Math.max(NODE_SIZE, ...recipients.map(endpoint => endpoint.width));
  const columnPositions = [];
  let columnLeft = 28 + initiatorWidth + columnGap;
  for (const width of columnWidths) {
    columnPositions.push(columnLeft);
    columnLeft += width + columnGap;
  }
  for (const node of nodes) node.x = columnPositions[node.column];
  for (const endpoint of initiators) endpoint.x = 28;
  for (const endpoint of recipients) endpoint.x = columnLeft;
  const width = columnLeft + recipientWidth + (flow.hasCycle ? 110 : 28);
  const height = Math.max(148, ...[...nodes, ...initiators, ...recipients].map(node => node.y + node.height + 36));
  const identityEdges = [
    ...initiators.flatMap(endpoint => endpoint.taskIds.map(id => ({ from: endpoint, to: nodeMap.get(id), type: 'identity', label: '发起任务' }))),
    ...recipients.flatMap(endpoint => endpoint.taskIds.map(id => ({ from: nodeMap.get(id), to: endpoint, type: 'notification', label: '结果通知' }))),
  ];
  return {
    nodes, initiators, recipients, identityEdges, width, height,
    compressed: nodes.some(node => rank.get(node.id) === MAX_RANK && flow.edges.some(edge => edge.to === node.id && rank.get(edge.from) === MAX_RANK)),
  };
}
