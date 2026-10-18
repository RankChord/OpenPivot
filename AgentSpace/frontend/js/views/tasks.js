import { api, operationKey } from '../api.js';
import { el, button, field, input, textarea, select, agentItems, agentName, pageHeader, panel, empty, statusBadge, when, modal, submit } from '../ui.js';
import { renderTaskDetails } from './task-details.js';
import { renderTaskGraph, getTaskFlows } from './task-graph.js';
import { coordinationMode, coordinationLabel } from '../task-graph-model.js';

/**
 * 创建任务只入队，真实执行依赖外部 Agent 轮询 task_claim。
 * 幂等 key 在表单打开时创建：网络错误后重试表单不会重复派单。
 */
export function enqueueTask(ctx, preset = {}) {
  if (!ctx.state.agents.some(agent => agent.active)) { ctx.navigate('agents'); ctx.toast('先注册并启用至少一个 Agent'); return; }
  const parentTask = ctx.state.tasks.find(task => task.id === preset.parent_task_id);
  if (parentTask && coordinationMode(parentTask) === 'fixed') { ctx.toast('固定编排的阶段已提前定义，请创建新流程安排其他工作。'); return; }
  const form = el('form', { class: 'modal-body' });
  const sender = select('sender', [{ value: '', label: '管理员 / 用户' }, ...agentItems(ctx)], preset.sender || '');
  const mode = select('collaboration_mode', [{ value: 'automatic', label: 'Agent 自动配合' }, { value: 'fixed', label: '固定编排' }], preset.collaboration_mode || 'automatic', { disabled: Boolean(preset.parent_task_id) });
  const modeHelp = el('p', { class: 'task-mode-explanation' });
  const assignee = select('assignee', agentItems(ctx), preset.assignee || ctx.state.actor || agentItems(ctx)[0]?.value, { required: true });
  const title = input('title', preset.title || '', { required: true, maxLength: 200, placeholder: '例如 编写后端 API 接口文档' });
  const instructions = textarea('instructions', preset.instructions || '', { required: true, rows: 6, maxLength: 16000, placeholder: '说明目标、约束、输入资料路径、成果路径与验收标准。' });
  const replyTo = select('reply_to', [{ value: '', label: '通知任务发起者（默认）' }, { value: '__operator', label: '管理员收件箱' }, ...agentItems(ctx)], preset.reply_to || '');
  const parent = input('parent_task_id', preset.parent_task_id || '', { placeholder: '可选，关联上级任务 ID', readOnly: Boolean(preset.parent_task_id) });
  const key = operationKey();
  const automatic = el('section', { class: 'task-mode-content' }, field('第一位执行 Agent', assignee), field('目标提示词与验收要求', instructions), field('上级任务 ID', parent, '关联上级用于组织实际流转记录；需要等待其他任务时另行登记依赖。'));
  const stageList = el('div', { class: 'workflow-stage-list' });
  const stageCount = el('span', { class: 'helper', role: 'status' });
  const stages = [];
  let stageSequence = 0;
  const addStage = button('＋ 添加阶段', () => createStage(), 'button button-small button-secondary', { 'data-action': 'add-stage' });
  const fixed = el('section', { class: 'task-mode-content', hidden: true },
    el('div', { class: 'workflow-stage-toolbar' }, el('strong', { text: '按顺序执行的阶段' }), stageCount), stageList, addStage);

  // 每个阶段保存自己的控件；排序只移动已有 DOM，已经填写的提示词不会丢失。
  function arrangeStages() {
    for (const [index, stage] of stages.entries()) {
      stage.heading.textContent = `阶段 ${index + 1}`;
      stage.up.disabled = index === 0;
      stage.down.disabled = index === stages.length - 1;
      stage.remove.disabled = stages.length === 1;
      stageList.append(stage.node);
      for (const control of stage.node.querySelectorAll('input, select, textarea')) control.disabled = mode.value !== 'fixed';
    }
    addStage.disabled = stages.length >= 32;
    stageCount.textContent = `${stages.length} / 32 个阶段 · 前一阶段完成后，下一阶段入队`;
  }
  function createStage() {
    if (stages.length >= 32) return;
    const number = ++stageSequence;
    const stage = {
      heading: el('h4'),
      assignee: select(`step_assignee_${number}`, agentItems(ctx), assignee.value, { required: true }),
      title: input(`step_title_${number}`, '', { required: true, maxLength: 200, placeholder: '例如 编写视频文案' }),
      instructions: textarea(`step_instructions_${number}`, '', { required: true, rows: 4, maxLength: 16000, placeholder: '本阶段的输入、任务提示词、交付要求。' }),
    };
    const move = direction => {
      const index = stages.indexOf(stage);
      const target = index + direction;
      if (target < 0 || target >= stages.length) return;
      [stages[index], stages[target]] = [stages[target], stages[index]];
      arrangeStages(); (direction < 0 ? stage.up : stage.down).focus();
    };
    stage.up = button('上移', () => move(-1), 'button button-small button-quiet', { 'aria-label': '阶段上移', 'data-action': 'move-stage-up' });
    stage.down = button('下移', () => move(1), 'button button-small button-quiet', { 'aria-label': '阶段下移', 'data-action': 'move-stage-down' });
    stage.remove = button('删除', () => { stages.splice(stages.indexOf(stage), 1); stage.node.remove(); arrangeStages(); }, 'button button-small button-quiet', { 'aria-label': '删除阶段', 'data-action': 'remove-stage' });
    stage.node = el('div', { class: 'workflow-stage' }, el('div', { class: 'workflow-stage-heading' }, stage.heading,
      el('div', { class: 'inline-actions' }, stage.up, stage.down, stage.remove)),
    el('div', { class: 'form-grid' }, field('执行 Agent', stage.assignee), field('阶段标题', stage.title)), field('阶段提示词', stage.instructions));
    stages.push(stage); arrangeStages();
  }

  form.append(el('div', { class: 'form-grid' }, field('任务发起者', sender), field('协作模式', mode, preset.parent_task_id ? '子任务继承上级的自动配合模式。' : '选择提前定义步骤，或让执行 Agent 自行决定后续协作。')),
    modeHelp, field('任务 / 流程标题', title), field('完成后通知', replyTo), automatic, fixed,
    el('p', { class: 'helper', text: '任务入队后，接入的 Agent 客户端仍需轮询领取。自动配合由 Agent 根据目标和成果判断下一位协作者，再调用 MCP 派发任务。' }));
  const dialog = modal('创建协作任务', form, true);
  const submitButton = el('button', { type: 'submit', class: 'button button-primary', text: '加入任务队列' });
  form.append(el('div', { class: 'form-actions' }, button('取消', dialog.close), submitButton));
  function changeMode() {
    const isFixed = mode.value === 'fixed';
    automatic.hidden = isFixed; fixed.hidden = !isFixed;
    for (const control of automatic.querySelectorAll('input, select, textarea')) control.disabled = isFixed;
    if (isFixed && !stages.length) createStage();
    arrangeStages();
    modeHelp.textContent = isFixed
      ? '固定编排：现在定义全部执行顺序和各阶段提示词。Agent 按既定路线执行，流程内不能临时增加阶段。'
      : 'Agent 自动配合：只创建当前任务。执行 Agent 可查看职责目录，根据提示词或工作结果自行选择下一位协作者；任务图随实际派发逐步生长。';
    submitButton.textContent = isFixed ? '创建固定流程' : '加入任务队列';
  }
  mode.addEventListener('change', changeMode); changeMode();
  form.addEventListener('submit', event => {
    event.preventDefault(); submit(form, async () => {
      const isFixed = mode.value === 'fixed';
      const args = { title: title.value.trim(), idempotency_key: key };
      if (replyTo.value) args.reply_to = replyTo.value;
      if (isFixed) args.steps = stages.map(stage => ({ assignee: stage.assignee.value, title: stage.title.value.trim(), instructions: stage.instructions.value.trim() }));
      else {
        Object.assign(args, { assignee: assignee.value, instructions: instructions.value.trim(), collaboration_mode: 'automatic' });
        if (parent.value.trim()) args.parent_task_id = parent.value.trim();
      }
      const result = await api.call(isFixed ? 'workflow_create' : 'task_enqueue', args, sender.value || null);
      dialog.close(); await ctx.refreshAfterWrite(); ctx.toast(`${isFixed ? '已创建固定流程' : '已派单'}：${result.task?.title || result.title || title.value}`);
    });
  });
}

/** 摘要卡片只给预览；此入口打开完整提示词、关联任务和文件版本。 */
export function showTask(ctx, task) {
  let close = () => {};
  const body = renderTaskDetails(ctx, task, {
    onLeave: () => close(),
    onSelect: id => {
      const related = ctx.state.tasks.find(item => item.id === id);
      if (related) { close(); showTask(ctx, related); }
    },
    onDelegate: () => { close(); delegateTask(ctx, task); },
  });
  const details = modal('任务详情', body, true);
  details.dialog.classList.add('task-detail-modal');
  close = details.close;
}

function delegateTask(ctx, task) {
  const enabled = ctx.state.agents.some(agent => agent.id === task.assignee && agent.active);
  enqueueTask(ctx, {
    sender: enabled ? task.assignee : '',
    reply_to: enabled ? task.assignee : '__operator',
    parent_task_id: task.id,
    collaboration_mode: coordinationMode(task),
  });
}

/**
 * 工作队列的图形/列表共用筛选与选择状态。
 * 圆头像、悬停胶囊和点击展开共用同一节点；完整详情由独立弹窗承载。
 * 筛选图形时保留同一流程的其他节点，避免把交接箭头截断；匹配节点高亮。
 * 定时刷新只更新本视图，跳过打开的编辑弹窗；离开页面时清理计时器。
 */
export function renderTasks(ctx) {
  const assignee = select('task-agent-filter', [{ value: '', label: '所有 Agent' }, ...agentItems(ctx, true)], ctx.state.taskAgentFilter || '');
  const status = select('task-status-filter', [{ value: '', label: '所有状态' },
    ...[['queued', '待领取'], ['leased', '执行中'], ['blocked', '等待依赖'], ['completed', '已完成'], ['failed', '失败'], ['cancelled', '已取消']]
      .map(([value, label]) => ({ value, label }))], ctx.state.taskStatusFilter || '');
  const coordination = select('task-mode-filter', [{ value: '', label: '所有协作模式' }, { value: 'automatic', label: 'Agent 自动配合' }, { value: 'fixed', label: '固定编排' }], ctx.state.taskModeFilter || '', { 'aria-label': '协作模式筛选' });
  const flow = select('task-flow-filter', [{ value: '', label: '所有任务流程' }], ctx.state.taskFlowFilter || '', { 'aria-label': '任务流程' });
  const search = input('search', ctx.state.taskSearch || '', { type: 'search', placeholder: '搜索任务标题 / ID', 'aria-label': '搜索任务' });
  const content = el('div', { class: 'task-view-content' });
  const count = el('p', { class: 'task-filter-count', role: 'status' });
  let disposed = false;
  let polling = false;
  let pollFailed = false;
  let lastDataSignature = '';
  ctx.state.taskPresentation ||= 'graph';

  function choose(id) {
    if (disposed) return;
    const previousId = ctx.state.taskSelectedId;
    const changed = previousId !== id;
    ctx.state.taskSelectedId = id;
    const graph = content.querySelector('.task-graph');
    // 选择只改变现有节点的几何与形态，保持同一头像 DOM 连续变形。
    if (ctx.state.taskPresentation === 'graph' && graph?.setSelection) graph.setSelection(id, { animate: changed });
    else update({ animateSelection: Boolean(id && changed) });
    // 收起后把键盘焦点留在原头像上，继续支持 Enter 和 Escape。
    const focusId = id || previousId;
    [...content.querySelectorAll('.task-graph-node')].find(node => node.dataset.taskId === focusId)?.focus({ preventScroll: true });
  }
  function details(id) {
    const task = ctx.state.tasks.find(item => item.id === id);
    if (task) showTask(ctx, task);
  }
  function presentation(mode) {
    ctx.state.taskPresentation = mode;
    update();
  }
  const graphButton = button('图形视图', () => presentation('graph'), 'task-view-toggle');
  const listButton = button('列表视图', () => presentation('list'), 'task-view-toggle');

  function update({ animateSelection = false } = {}) {
    if (disposed) return;
    ctx.state.taskAgentFilter = assignee.value;
    ctx.state.taskStatusFilter = status.value;
    ctx.state.taskModeFilter = coordination.value;
    ctx.state.taskSearch = search.value;
    const flows = getTaskFlows(ctx.state.tasks);
    const chosenFlow = flow.value || ctx.state.taskFlowFilter || '';
    flow.replaceChildren(el('option', { value: '', text: '所有任务流程' }),
      ...flows.map(item => el('option', { value: item.id, text: item.title + ' · ' + item.tasks.length + ' 个节点' })));
    flow.value = flows.some(item => item.id === chosenFlow) ? chosenFlow : '';
    ctx.state.taskFlowFilter = flow.value;

    const query = search.value.trim().toLowerCase();
    const matches = ctx.state.tasks.filter(task =>
      (!assignee.value || task.assignee === assignee.value) &&
      (!status.value || task.status === status.value) &&
      (!coordination.value || coordinationMode(task) === coordination.value) &&
      (!query || (task.title + ' ' + task.id).toLowerCase().includes(query)));
    const matchingIds = new Set(matches.map(task => task.id));
    const visibleFlows = flows.filter(item =>
      (!flow.value || item.id === flow.value) && item.taskIds.some(id => matchingIds.has(id)));
    const visibleTasks = visibleFlows.flatMap(item => item.tasks);
    const visibleIds = new Set(visibleTasks.map(task => task.id));
    const visibleMatches = matches.filter(task => visibleIds.has(task.id));
    if (ctx.state.taskSelectedId && !visibleIds.has(ctx.state.taskSelectedId)) ctx.state.taskSelectedId = null;

    const mode = ctx.state.taskPresentation;
    for (const [control, value] of [[graphButton, 'graph'], [listButton, 'list']]) {
      control.classList.toggle('active', mode === value);
      control.setAttribute('aria-pressed', mode === value ? 'true' : 'false');
    }
    count.textContent = visibleMatches.length + ' 个匹配任务 · ' + visibleFlows.length + ' 条流程';
    const noTasks = empty('没有符合条件的任务', '创建任务，或调整筛选条件查看已有流程。');
    const active = document.activeElement;
    const focusedTask = content.contains(active) ? active.closest('[data-task-id]') : null;
    const focusTarget = focusedTask ? { id: focusedTask.dataset.taskId, action: active.dataset.action,
      avatar: active.classList.contains('task-graph-node') } : null;
    const summaryBody = content.querySelector('.task-graph-summary .task-graph-summary-body');
    const summaryScroll = summaryBody ? { id: summaryBody.closest('[data-task-id]').dataset.taskId, top: summaryBody.scrollTop } : null;
    // 长流程可以独立滚动；更新状态或展开头像后继续停留在当前阅读位置。
    const scrollPositions = new Map([...content.querySelectorAll('.task-flow-card')].map(card => {
      const viewport = card.querySelector('.task-graph-viewport');
      return [card.dataset.flowId, { left: viewport.scrollLeft, top: viewport.scrollTop }];
    }));
    content.querySelector('.task-graph')?.destroy?.();
    content.replaceChildren(!visibleTasks.length ? noTasks : mode === 'graph'
      ? renderTaskGraph(ctx, visibleTasks, { selectedId: ctx.state.taskSelectedId, onSelect: choose,
        onDetails: details, animateSelection, highlightIds: matchingIds })
      : taskTable(ctx, visibleMatches, ctx.state.taskSelectedId, id => { choose(id); details(id); }));
    for (const card of content.querySelectorAll('.task-flow-card')) {
      const previous = scrollPositions.get(card.dataset.flowId);
      if (!previous) continue;
      const viewport = card.querySelector('.task-graph-viewport');
      viewport.scrollLeft = previous.left; viewport.scrollTop = previous.top;
    }
    const selectedSummary = content.querySelector('.task-graph-summary');
    if (summaryScroll && selectedSummary?.dataset.taskId === summaryScroll.id) {
      selectedSummary.querySelector('.task-graph-summary-body').scrollTop = summaryScroll.top;
    }
    // Agent 更新状态也会重建节点；保留键盘焦点，避免用户等待后无法继续 Enter。
    if (focusTarget) {
      const target = focusTarget.avatar
        ? [...content.querySelectorAll('.task-graph-node')].find(node => node.dataset.taskId === focusTarget.id)
        : selectedSummary?.dataset.taskId === focusTarget.id
          ? [...selectedSummary.querySelectorAll('[data-action]')].find(control => control.dataset.action === focusTarget.action) : null;
      target?.focus({ preventScroll: true });
    }
    lastDataSignature = JSON.stringify([ctx.state.tasks, ctx.state.agents]);
  }

  flow.addEventListener('change', () => { ctx.state.taskFlowFilter = flow.value; update(); });
  assignee.addEventListener('change', update);
  status.addEventListener('change', update);
  coordination.addEventListener('change', update);
  search.addEventListener('input', update);
  update();

  // 不触发整页 render，保留筛选输入和选中头像；未变化时不重播展开动画。
  const timer = setInterval(async () => {
    if (disposed || polling || document.hidden || document.querySelector('dialog[open]') || content.querySelector('.task-graph')?.isAnimating?.()) return;
    polling = true;
    try {
      await ctx.refresh(false);
      // 连接恢复时，即使数据未变，也应清除先前的刷新错误提示。
      if (!disposed && !document.querySelector('dialog[open]') && !content.querySelector('.task-graph')?.isAnimating?.() &&
        (pollFailed || lastDataSignature !== JSON.stringify([ctx.state.tasks, ctx.state.agents]))) update();
      pollFailed = false;
    } catch (error) {
      pollFailed = true;
      if (!disposed) count.textContent = '自动刷新失败：' + error.message;
    } finally { polling = false; }
  }, 5000);
  const dismissSummary = event => {
    if (event.key !== 'Escape' || document.querySelector('dialog[open]')) return;
    const close = content.querySelector('.task-graph-summary [data-action="collapse"]');
    if (close) { event.preventDefault(); close.click(); }
  };
  document.addEventListener('keydown', dismissSummary);
  ctx.onDispose(() => {
    disposed = true; clearInterval(timer);
    content.querySelector('.task-graph')?.destroy?.();
    document.removeEventListener('keydown', dismissSummary);
  });

  return el('div', {},
    pageHeader('TASK COORDINATION', '任务与交付', '固定编排展示预设步骤；Agent 自动配合展示实际派发记录。悬停姓名胶囊，点击展开提示词与交付。',
      [button('＋ 创建任务', () => enqueueTask(ctx), 'button button-primary')]),
    el('section', { class: 'panel task-workspace' },
      el('div', { class: 'task-workspace-toolbar' },
        el('div', { class: 'task-view-switch', 'aria-label': '任务展示方式' }, graphButton, listButton),
        el('span', { class: 'helper', text: '每 5 秒更新状态 · 编辑弹窗打开时暂停' })),
      el('div', { class: 'filter-bar task-filters' }, flow, coordination, assignee, status, search),
      el('div', { class: 'task-view-caption' }, count,
        el('span', { class: 'helper', text: '图形筛选保留同流程上下文，淡化不匹配节点。' })),
      content));
}

function taskTable(ctx, tasks, selectedId, onSelect) {
  const ordered = [...tasks].sort((a, b) => b.updated_at - a.updated_at);
  return el('div', { class: 'table-scroll' }, el('table', { class: 'data-table task-list-table' },
    el('thead', {}, el('tr', {}, ['任务 / 要求', '协作模式', '执行 Agent', '状态', '尝试', '更新']
      .map(title => el('th', { text: title })))),
    el('tbody', {}, ordered.map(task =>
      el('tr', { class: task.id === selectedId ? 'task-row-selected' : '' },
        el('td', {}, button(task.title, () => onSelect(task.id), 'table-title'), el('small', { text: task.id.slice(0, 8) })),
        el('td', {}, el('span', { class: `badge mode-${coordinationMode(task)}`, text: coordinationLabel(task) })),
        el('td', { text: agentName(ctx, task.assignee) }),
        el('td', {}, statusBadge(task.status)),
        el('td', { text: task.attempt + '/' + task.max_attempts }),
        el('td', { class: 'nowrap', text: when(task.updated_at) }))))));
}
