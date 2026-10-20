// 插件往 im 页面加的内容（施工 I-8 下，设计 01 第四节第 5 条）：照核心的界面块（widgets.md）画。首页卡片、会话浮窗的一页、整页都用它。
// 内容由宿主照核心的那一份查过、照语言挑好字（plugins.widget），这里一律当字画（textContent）。
//
// - 段：heading、text、pairs、progress、list、buttons；整页另有 sections、form（widget-form.js）。认不出的段不画。
// - 动作：{call, params} 经宿主调这个插件登记的方法（plugins.call），调完重取一次内容；{open} 打开这个插件的那个整页；{copy} 写进剪贴板。
// - 语气 tone、状态 state 照 im 自己的颜色画。
import { element, say } from '../../shell/i18n.js';
import { copy, toast } from '../../shell/dialog.js';
import { drawForm, drawSections } from './widget-form.js';

/** 画一块内容 `content`（段的数组）：`context` 是 { call(method, params), open(id), refresh() }。 */
export function drawWidget(content, context) {
  const parts = [];
  for (const part of Array.isArray(content) ? content : []) {
    const drawn = block(part, context);
    if (drawn) parts.push(drawn);
  }
  return parts;
}

/** 一段。 */
export function block(part, context) {
  switch (part?.type) {
    case 'heading':
      return element('h4', 'w-heading', part.text);
    case 'text':
      return element('p', `w-text ${tone(part.tone)}`, part.text);
    case 'pairs': {
      const box = element('div', 'w-pairs');
      for (const pair of part.items || []) {
        const row = element('div', 'meta-row');
        row.append(element('span', null, pair.label), element('b', tone(pair.tone), pair.value));
        box.append(row);
      }
      return box;
    }
    case 'progress':
      return progress(part);
    case 'list':
      return list(part.items || [], context);
    case 'buttons': {
      const box = element('div', 'w-buttons');
      for (const item of part.items || []) {
        const button = element('button', 'btn', item.label);
        button.addEventListener('click', () => act(item.action, context, button));
        box.append(button);
      }
      return box;
    }
    case 'sections':
      return drawSections(part, context);
    case 'form':
      return drawForm(part, context);
    default:
      return null;
  }
}

/** 进度条：0 到 1，`mark` 标一处，`text` 写在上面。 */
function progress(part) {
  const box = element('div', 'w-progress');
  if (part.text) box.append(element('small', null, part.text));
  const bar = element('div', 'w-bar');
  const fill = element('i');
  fill.style.width = `${Math.round(Math.min(1, Math.max(0, Number(part.value) || 0)) * 100)}%`;
  bar.append(fill);
  if (typeof part.mark === 'number') {
    const mark = element('b');
    mark.style.left = `${Math.round(Math.min(1, Math.max(0, part.mark)) * 100)}%`;
    bar.append(mark);
  }
  box.append(bar);
  return box;
}

/** 列表：状态点、标题、小字、右边一段（字或几段带语气的字）；有动作的能点。 */
function list(items, context) {
  const box = element('div', 'w-list');
  for (const item of items) {
    const row = element(item.action ? 'button' : 'div', 'w-item');
    if (item.state) row.append(element('span', `w-state w-${item.state}`));
    const label = element('span', 'w-label', item.title);
    if (item.detail) label.append(element('small', null, item.detail));
    row.append(label);
    if (Array.isArray(item.right)) {
      const right = element('span', 'w-right');
      for (const piece of item.right) right.append(element('span', tone(piece.tone), piece.text));
      row.append(right);
    } else if (item.right) {
      row.append(element('span', 'w-right', item.right));
    }
    if (item.action) row.addEventListener('click', () => act(item.action, context, row));
    box.append(row);
  }
  return box;
}

/** 做一个动作（模块的说明）；`from` 是点的那一个（做的时候按不下去）。 */
export async function act(action, context, from) {
  if (!action) return;
  if (typeof action.copy === 'string') {
    await copy(action.copy);
    return;
  }
  if (typeof action.open === 'string') {
    context.open(action.open);
    return;
  }
  if (typeof action.call !== 'string') return;
  if (from) from.disabled = true;
  try {
    await context.call(action.call, action.params);
  } catch {
    toast(say('plugins-page/action-failed'));
  }
  if (from) from.disabled = false;
  context.refresh();
}

/** 语气的样式。 */
export function tone(name) {
  return ['dim', 'accent', 'good', 'warn', 'bad'].includes(name) ? `tone-${name}` : '';
}
