import { api } from '../api.js';
import { el, button, field, input, textarea, select, agentItems, pageHeader, panel, empty, bytes, when, modal, submit } from '../ui.js';

/** SQLite 文件不是本机磁盘。路径是工作区逻辑路径；更新必须带 expected_version 防止覆盖他人交付。 */
export function renderFiles(ctx) {
  const prefix = input('prefix', ctx.state.filePrefix || '', { placeholder: 'shared/ 或 agents/某身份/memory/', 'aria-label': '文件路径前缀' });
  const identity = select('file_actor', [{ value: '', label: '管理员 · 所有文件' }, ...agentItems(ctx)], ctx.state.fileActor || '');
  const shortcuts = [{ value: '', label: '全部文件' }, { value: 'shared/', label: '共享资料' }];
  for (const agent of ctx.state.agents) shortcuts.push({ value: agent.knowledge_prefix, label: `${agent.name} / 知识库` }, { value: agent.memory_prefix, label: `${agent.name} / 记忆` });
  const directory = select('directory', shortcuts, ctx.state.filePrefix || '');
  const listHost = el('div', { class: 'file-list-host' }, el('p', { class: 'loading', text: '读取文件目录…' }));
  const editorHost = el('div', { class: 'editor-host' }, empty('选择一个文件', '共享资料可以交接，私有知识与记忆跟随稳定 Agent 身份。'));
  let currentFile = null;
  let readSequence = 0;
  let listSequence = 0;
  async function loadList() {
    const sequence = ++listSequence;
    ctx.state.filePrefix = prefix.value; ctx.state.fileActor = identity.value;
    try {
      const result = await api.call('file_list', { prefix: prefix.value }, identity.value || null);
      if (sequence !== listSequence) return;
      ctx.state.files = result.files;
      listHost.replaceChildren(result.files.length ? el('div', { class: 'file-list' }, result.files.map(file => fileRow(file))) : empty('该目录还没有文件', '新建文本或上传文件，保存到 shared/ 或 Agent 自己的目录。'));
    } catch (error) { listHost.replaceChildren(el('p', { class: 'notice notice-error', role: 'alert', text: error.message })); }
  }
  function fileRow(file) {
    return el('button', { type: 'button', class: `file-row${currentFile?.path === file.path ? ' selected' : ''}`, onclick: () => loadFile(file.path) }, el('span', { class: 'file-icon', text: '▤' }), el('div', {}, el('strong', { text: file.path }), el('span', { text: `v${file.version} · ${bytes(file.size)} · ${when(file.created_at)}` })));
  }
  async function loadFile(path, version = null) {
    const sequence = ++readSequence;
    editorHost.replaceChildren(el('p', { class: 'loading', text: '读取文件版本…' }));
    try {
      const args = { path }; if (version) args.version = Number(version);
      const file = await api.call('file_read', args, identity.value || null);
      if (sequence !== readSequence) return;
      currentFile = file; ctx.state.filePath = path; ctx.state.fileVersion = version;
      editorHost.replaceChildren(editor(file));
      listHost.querySelectorAll('.file-row').forEach(row => row.classList.toggle('selected', row.querySelector('strong')?.textContent === path));
    } catch (error) { if (sequence === readSequence) editorHost.replaceChildren(el('p', { class: 'notice notice-error', role: 'alert', text: error.message })); }
  }
  function editor(file) {
    const content = textarea('content', file.content || '', { rows: 18, class: 'file-editor', spellcheck: false, readOnly: file.content == null });
    const revision = input('revision', file.version, { type: 'number', min: 1, 'aria-label': '要读取的文件版本' });
    const form = el('form', { class: 'file-editor-form' }, el('div', { class: 'editor-heading' }, el('div', {}, el('h3', { text: file.path }), el('p', { class: 'helper', text: `当前显示 v${file.version} · ${bytes(file.size)} · ${file.created_by}` })), button('下载', () => download(file), 'button button-small button-secondary')), el('div', { class: 'revision-bar' }, el('span', { text: '读取历史版本' }), revision, button('读取', () => loadFile(file.path, revision.value), 'button button-small button-secondary'), button('最新版本', () => loadFile(file.path), 'button button-small button-quiet')), file.content == null ? el('p', { class: 'notice', text: '这是二进制文件，可以下载原始内容；编辑文本请选择 UTF-8 文件。' }) : content, el('div', { class: 'editor-footer' }, el('span', { class: 'helper', text: `保存使用 expected_version = ${file.version}；有新版本时会提示冲突。` }), el('button', { type: 'submit', class: 'button button-primary', text: '保存为下一版本', disabled: file.content == null })), el('code', { class: 'checksum', text: `SHA-256 ${file.sha256}` }));
    form.addEventListener('submit', event => { event.preventDefault(); submit(form, async () => {
      await api.call('file_write', { path: file.path, content: content.value, expected_version: file.version }, identity.value || null);
      await loadList(); await loadFile(file.path); await ctx.refreshAfterWrite(false); ctx.toast('文件新版本已保存');
    }); }); return form;
  }
  directory.addEventListener('change', () => { prefix.value = directory.value; loadList(); });
  identity.addEventListener('change', () => { ctx.state.fileActor = identity.value; prefix.value = identity.value ? `agents/${identity.value}/` : ''; loadList(); currentFile = null; editorHost.replaceChildren(empty('已切换访问身份', '选择文件读取；后端会验证该身份的目录访问权限。')); });
  const filter = el('form', { class: 'filter-bar file-filter' }, identity, directory, prefix, el('button', { type: 'submit', class: 'button button-secondary', text: '读取目录' }));
  filter.addEventListener('submit', event => { event.preventDefault(); loadList(); });
  const result = el('div', {}, pageHeader('KNOWLEDGE & ARTIFACTS', '文件、知识与记忆', '共享资料服务协作，私有文档保存每个 Agent 的长期上下文。', [button('＋ 新建文本', () => newFile(ctx, identity.value, prefix.value, false, async path => { await loadList(); await loadFile(path); })), button('↑ 上传文件', () => newFile(ctx, identity.value, prefix.value, true, async path => { await loadList(); await loadFile(path); }), 'button button-primary')]), panel('工作区目录', '通过路径前缀筛选；管理员可以管理所有目录，Agent 只可访问共享目录与自己的私有目录。', el('div', {}, filter, el('div', { class: 'files-layout' }, listHost, editorHost))));
  loadList(); if (ctx.state.filePath) loadFile(ctx.state.filePath, ctx.state.fileVersion);
  return result;
}

function newFile(ctx, actor, prefix, binary, onSaved) {
  const path = input('path', prefix || 'shared/', { required: true, placeholder: 'shared/docs/spec.md' });
  const version = input('expected_version', '0', { required: true, type: 'number', min: 0 });
  const content = binary ? input('upload', '', { type: 'file', required: true }) : textarea('content', '', { rows: 12, spellcheck: false, placeholder: '# 项目开发说明\n\n记录目标、约束与交付标准。' });
  const form = el('form', { class: 'modal-body' }, field('工作区文件路径', path), field('预期当前版本', version, '新文件填 0；覆盖已有文件需要填写当前版本，服务端会保留所有历史版本。'), field(binary ? '选择本地文件（最大 1 MiB）' : 'UTF-8 文本内容', content));
  const dialog = modal(binary ? '上传文件' : '新建文本文件', form, true);
  form.append(el('div', { class: 'form-actions' }, button('取消', dialog.close), el('button', { type: 'submit', class: 'button button-primary', text: '保存到工作区' })));
  content.addEventListener('change', () => { if (binary && content.files[0] && path.value.endsWith('/')) path.value += content.files[0].name; });
  form.addEventListener('submit', event => { event.preventDefault(); submit(form, async () => {
    const args = { path: path.value.trim(), expected_version: Number(version.value) };
    if (binary) {
      const file = content.files[0]; if (!file) throw new Error('请选择文件');
      if (file.size > 1024 * 1024) throw new Error('演示版单文件最大 1 MiB');
      const data = new Uint8Array(await file.arrayBuffer());
      // 分块转换，避免大文件调用 String.fromCharCode(...data) 导致栈溢出。
      const chunks = []; for (let i = 0; i < data.length; i += 32768) chunks.push(String.fromCharCode(...data.subarray(i, i + 32768)));
      args.content_base64 = btoa(chunks.join(''));
    } else args.content = content.value;
    await api.call('file_write', args, actor || null); dialog.close(); await onSaved(args.path); await ctx.refreshAfterWrite(false); ctx.toast('文件已写入工作区');
  }); });
}

function download(file) {
  const data = Uint8Array.from(atob(file.content_base64), character => character.charCodeAt(0));
  const url = URL.createObjectURL(new Blob([data], { type: 'application/octet-stream' }));
  const anchor = el('a', { href: url, download: file.path.split('/').pop() }); document.body.append(anchor); anchor.click(); anchor.remove(); setTimeout(() => URL.revokeObjectURL(url), 1000);
}
