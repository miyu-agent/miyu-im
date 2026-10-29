// 整页里能改的两种段（施工 I-8 下，核心 widgets.md「整页里能改」）：sections（左边一列项、右边选中那一项的几段）、form（几格加一个
// 保存）。选中哪一项、表单里改了还没存的格记在 context.state 里：重取内容（存了、点了动作以后）时照旧，同 id 的表单人改过的格留着。
//
// - 保存只交这次改过的格（改回默认的交 null），经宿主调 submit.call，参数是 submit.params 合上 {values}；回 {saved: false, problems}
//   的写在那一格下面（不在这张表上的写在表单底下），{saved: false, message} 的写在表单底下，填的值不丢。
// - 加一项：有选项的先选再加，回 {id} 的选中它。
import { element, icon, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { tagInput } from '../../shell/tags.js';
import { block, tone } from './widgets.js';

/** sections 段。 */
export function drawSections(part, context) {
  const state = context.state;
  const items = part.items || [];
  if (!items.some((item) => item.id === state.selected)) state.selected = items[0]?.id ?? null;
  const box = element('div', 'w-sections');
  const side = element('div', 'w-sec-list');
  const body = element('div', 'w-sec-body');
  for (const item of items) {
    const row = element('button', item.id === state.selected ? 'w-sec-item on' : 'w-sec-item');
    if (item.state) row.append(element('span', `w-state w-${item.state}`));
    const label = element('span', 'w-label', item.title);
    if (item.subtitle) label.append(element('small', null, item.subtitle));
    row.append(label);
    if (item.badge) {
      const badge = typeof item.badge === 'string' ? { text: item.badge } : item.badge;
      row.append(element('span', `w-badge ${tone(badge.tone)}`, badge.text));
    }
    row.addEventListener('click', () => {
      state.selected = item.id;
      context.redraw();
    });
    side.append(row);
  }
  if (part.add) side.append(adding(part.add, context));
  const chosen = items.find((item) => item.id === state.selected);
  for (const inner of chosen?.blocks || []) {
    const drawn = block(inner, context);
    if (drawn) body.append(drawn);
  }
  box.append(side, body);
  return box;
}

/** 加一项：选项、按钮。 */
function adding(add, context) {
  const box = element('div', 'w-add');
  let pick = null;
  if (Array.isArray(add.options) && add.options.length > 0) {
    pick = element('select', 'select');
    for (const option of add.options) {
      const one = element('option', null, option.label);
      one.value = option.value;
      pick.append(one);
    }
    box.append(pick);
  }
  const button = element('button', 'btn');
  button.append(icon('i-plus'), document.createTextNode(add.label));
  button.addEventListener('click', async () => {
    button.disabled = true;
    try {
      const answer = await context.call(add.call, pick ? { value: pick.value } : {});
      if (typeof answer?.id === 'string') context.state.selected = answer.id;
    } catch {
      toast(say('plugins-page/action-failed'));
    }
    button.disabled = false;
    context.refresh();
  });
  box.append(button);
  return box;
}

/** form 段。 */
export function drawForm(part, context) {
  context.state.drafts ??= {};
  const drafts = (context.state.drafts[part.id] ??= {});
  const box = element('div', 'w-form');
  const failures = new Map();
  const save = element('button', 'btn primary', part.submit?.label || say('plugins-page/save'));
  for (const field of part.fields || []) {
    const failed = element('div', 'row-failed');
    failures.set(field.key, failed);
    const row = element('div', 'row');
    const label = element('div', 'label', field.label);
    if (field.hint) label.append(element('small', null, field.hint));
    const value = field.key in drafts ? drafts[field.key] : field.value;
    // `quiet` 的（标签当场画上了）不重画整张表，只让「保存」按得下去。
    const put = (next, quiet) => {
      drafts[field.key] = next;
      if (quiet) save.disabled = false;
      else context.redraw();
    };
    const side = element('span', 'inline');
    side.append(control(field, value, put));
    if (value !== null && value !== undefined && field.default) {
      const reset = element('button', 'btn ghost plug-reset', say('plugins-page/reset'));
      reset.addEventListener('click', () => put(null));
      side.append(reset);
    }
    row.append(label, side);
    const wrap = element('div', 'field');
    wrap.append(row, failed);
    box.append(wrap);
  }
  const bottom = element('div', 'row-failed');
  save.disabled = Object.keys(drafts).length === 0;
  save.addEventListener('click', async () => {
    save.disabled = true;
    bottom.textContent = '';
    for (const failed of failures.values()) failed.textContent = '';
    let answer;
    try {
      answer = await context.call(part.submit.call, { ...(part.submit.params || {}), values: { ...drafts } });
    } catch {
      bottom.textContent = say('failed/save');
      save.disabled = false;
      return;
    }
    if (answer?.saved) {
      context.state.drafts[part.id] = {};
      context.refresh();
      return;
    }
    for (const problem of answer?.problems || []) {
      (failures.get(problem.key) || bottom).textContent = problem.message;
    }
    if (answer?.message) bottom.textContent = answer.message;
    save.disabled = false;
  });
  const foot = element('div', 'w-form-foot');
  foot.append(bottom, save);
  box.append(foot);
  return box;
}

/** 一格的控件：值 `null` 的照默认画淡的。 */
function control(field, value, put) {
  const fallback = field.default?.value;
  const shown = value ?? fallback ?? null;
  switch (field.control) {
    case 'toggle': {
      const button = element('button', shown ? 'switch on' : 'switch');
      if (value === null) button.classList.add('inherited');
      button.append(element('i'));
      button.addEventListener('click', () => put(!shown));
      return button;
    }
    case 'select': {
      const pick = element('select', value === null ? 'select inherited' : 'select');
      if (value === null && fallback === undefined) pick.append(element('option', null, field.default?.text || ''));
      for (const option of field.options || []) {
        const one = element('option', null, option.label);
        one.value = option.value;
        one.selected = option.value === shown;
        pick.append(one);
      }
      pick.addEventListener('change', () => put(pick.value));
      return pick;
    }
    case 'number': {
      const input = element('input', 'input num');
      input.type = 'number';
      if (field.min !== undefined) input.min = field.min;
      if (field.max !== undefined) input.max = field.max;
      input.value = value ?? '';
      input.placeholder = field.default?.text || '';
      input.addEventListener('change', () => {
        const typed = input.value.trim();
        const number = Number(typed);
        put(typed === '' ? null : field.integer ? Math.trunc(number) : number);
      });
      return input;
    }
    case 'textarea': {
      const area = element('textarea', 'area');
      area.value = value ?? '';
      area.placeholder = field.default?.text || field.placeholder || '';
      area.addEventListener('change', () => put(area.value === '' ? null : area.value));
      return area;
    }
    case 'list':
      // 标签当场画上（shell/tags.js），只记进改了的格、不重画整张表（重画会把正在填的那一格换掉）。
      return tagInput({
        items: Array.isArray(value) ? value : null,
        inherited: Array.isArray(fallback) ? fallback : [],
        placeholder: field.placeholder,
        save: (next) => put(next, true),
      });
    default: {
      const input = element('input', 'input plug-text');
      input.value = value ?? '';
      input.placeholder = field.default?.text || field.placeholder || '';
      input.addEventListener('change', () => put(input.value.trim() === '' ? null : input.value.trim()));
      return input;
    }
  }
}
