// 插件往 im 页面加的内容（施工 I-8 下，设计 01 第四节第 5 条）：照核心的界面块（widgets.md）画。首页卡片、会话浮窗的一页、整页都用它。
// 内容由宿主照核心的那一份查过、照语言挑好字（plugins.widget），这里一律当字画（textContent）。
//
// - 段：heading、text、pairs、progress、list、buttons；整页另有 sections、form（widget-form.js）。认不出的段不画。
// - 动作：{call, params} 经宿主调这个插件登记的方法（plugins.call），调完重取一次内容；{open} 打开这个插件的那个整页；{copy} 写进剪贴板。
// - 语气 tone、状态 state 照 im 自己的颜色画。
// - 会话浮窗的一页、整页（`context.cards`，设计 11 G1，施工 I-166）：一个 heading 起一张卡片（shell/sect.js），它下面的段放进去，
//   第一个 heading 前面的在一张没名字的卡里。首页卡片本身就是一张卡，照旧平铺。
// - 列表超过 8 条的头上一个搜索框、20 条以后「再显示」（shell/listing.js）：照看得见的字找，核心的界面块不用多写什么。
import { element, say } from '../../shell/i18n.js';
import { copy, toast } from '../../shell/dialog.js';
import { listing } from '../../shell/listing.js';
import { sect } from '../../shell/sect.js';
import { drawForm, drawSections } from './widget-form.js';

/** 列表多于这几条才给搜索框。 */
const SEARCHABLE = 8;

/** 画一块内容 `content`（段的数组）：`context` 是 { call(method, params), open(id), refresh(), cards? }。 */
export function drawWidget(content, context) {
  context.lists = 0;
  return grouped(Array.isArray(content) ? content : [], context);
}

/** 几段：`context.cards` 的照 heading 分卡片，别的平铺。 */
export function grouped(parts, context) {
  const drawn = [];
  let card = null;
  for (const part of parts) {
    if (context.cards && part?.type === 'heading') {
      card = sect(part.text);
      drawn.push(card.box);
      continue;
    }
    const one = block(part, context);
    if (!one) continue;
    // 左边一列项、右边几段的（sections）自己分卡片，不再套一层。
    if (!context.cards || part.type === 'sections') drawn.push(one);
    else {
      if (!card) {
        card = sect(null);
        drawn.push(card.box);
      }
      card.body.append(one);
    }
  }
  return drawn;
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

/** 列表：状态点、标题、小字、右边一段（字或几段带语气的字）；有动作的能点。长的能搜、分页（模块的说明）。 */
function list(items, context) {
  const rows = items.map((item) => ({ node: listRow(item, context), text: listText(item) }));
  if (rows.length <= SEARCHABLE) {
    const box = element('div', 'w-list');
    box.append(...rows.map((row) => row.node));
    return box;
  }
  // 搜了什么、显示到几条：这一块重画（点了动作、存了表单以后）时照旧，照它是这一块里第几个列表记。
  context.state.lists ??= {};
  const state = (context.state.lists[context.lists++] ??= {});
  const box = listing(rows, { search: say('listing/search'), state });
  box.classList.add('w-list');
  return box;
}

/** 列表的一行。 */
function listRow(item, context) {
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
  return row;
}

/** 一行搜索照着找的字：标题、小字、右边。 */
function listText(item) {
  const right = Array.isArray(item.right) ? item.right.map((piece) => piece.text).join(' ') : item.right || '';
  return [item.title, item.detail, right].filter(Boolean).join(' ');
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
