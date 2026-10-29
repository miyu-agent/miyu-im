// 插件自己的设置（施工 I-9 中，设计 01 第四节第 5 条；照样板插件详情的「设置」）：插件清单里声明的配置项（核心 config.schema 里
// package 是它的），所有会话共用。一项一行：名字、说明、控件；照核心给的 control、type 挑控件：开关、下拉、数、一行字、几行字、列表、
// 下拉加「其他」、密钥（只说设没设，粘贴新的换上）。改了当场存（plugins.settings.set），存不成的写在那一行下面；和默认不一样的给
// 「恢复默认」。值写成引用（{ env = … } 这类）的不在这里改。
import { element, say } from '../../shell/i18n.js';
import { secretInput } from '../../shell/secret.js';
import { tagInput } from '../../shell/tags.js';

/** 插件 `package` 的设置：先画「读取中」，读到了换上。 */
export function drawSettings(packageId, request) {
  const box = element('div', 'plug-settings');
  const load = async () => {
    let got;
    try {
      got = await request('plugins.settings.get', { package: packageId });
    } catch {
      box.replaceChildren(element('p', 'faint', say('plugins-page/settings-failed')));
      return;
    }
    const items = (got.items || []).filter((item) => !item.hidden);
    if (items.length === 0) {
      box.replaceChildren(element('p', 'faint', say('plugins-page/no-settings')));
      return;
    }
    // 存了重读一遍；列表（标签当场画上了）正接着填的不重读，重读会把正在填的那一格换掉。
    const save = async (key, change, failed) => {
      failed.textContent = '';
      try {
        await request('plugins.settings.set', { package: packageId, changes: [{ key, ...change }] });
      } catch {
        failed.textContent = say('failed/invalid');
        return;
      }
      const typing = box.contains(document.activeElement) && document.activeElement.matches('.tag-add');
      if (!typing) load();
    };
    box.replaceChildren(...items.map((item) => row(item, got.values?.[item.key] ?? null, save)));
    box.append(element('p', 'faint plug-settings-note', say('plugins-page/settings-shared')));
  };
  box.append(element('p', 'faint', say('plugins-page/loading')));
  load();
  return box;
}

/** 一项一行。 */
function row(item, value, save) {
  const line = element('div', 'row');
  const label = element('div', 'label', item.name || item.key);
  if (item.description) label.append(element('small', null, item.description));
  const failed = element('div', 'row-failed');
  const put = (change) => save(item.key, change, failed);
  const side = element('span', 'inline');
  side.append(control(item, value, put));
  const differs = item.type !== 'secret' && value !== null && JSON.stringify(value) !== JSON.stringify(item.default ?? null);
  if (differs) {
    const reset = element('button', 'btn ghost plug-reset', say('plugins-page/reset'));
    reset.addEventListener('click', () => put({ unset: true }));
    side.append(reset);
  }
  line.append(label, side);
  const wrap = element('div', 'field');
  wrap.append(line, failed);
  return wrap;
}

/** 照 `item` 的控件、类型挑一个控件；`put` 交一次改动（`{value}`、`{unset}`）。 */
function control(item, value, put) {
  if (item.type === 'secret') return secret(value, put);
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    return element('span', 'faint', say('plugins-page/written-elsewhere'));
  }
  const kind = item.control || (item.type === 'bool' ? 'toggle' : item.options ? 'select' : 'text');
  switch (kind) {
    case 'toggle': {
      const button = element('button', value ? 'switch on' : 'switch');
      button.append(element('i'));
      button.addEventListener('click', () => put({ value: !value }));
      return button;
    }
    case 'select':
      return select(item.options || [], value, (picked) => put({ value: picked }));
    case 'combo':
      return combo(item.suggestions || [], value, put);
    case 'number': {
      const input = element('input', 'input num');
      input.type = 'number';
      if (item.min !== undefined) input.min = item.min;
      if (item.max !== undefined) input.max = item.max;
      input.value = value ?? '';
      input.addEventListener('change', () => {
        const typed = input.value.trim();
        put(typed === '' ? { unset: true } : { value: Number(typed) });
      });
      return input;
    }
    case 'textarea': {
      const area = element('textarea', 'area');
      area.value = value ?? '';
      area.addEventListener('change', () => put(area.value === '' ? { unset: true } : { value: area.value }));
      return area;
    }
    case 'list':
      return list(Array.isArray(value) ? value : [], put);
    default: {
      const input = element('input', 'input plug-text');
      input.value = value ?? '';
      input.addEventListener('change', () => put(input.value.trim() === '' ? { unset: true } : { value: input.value.trim() }));
      return input;
    }
  }
}

/** 下拉：`options` 是 [{value, name, available?}]。 */
function select(options, value, pick) {
  const shown = element('select', 'select');
  for (const option of options) {
    const one = element('option', null, option.name ?? String(option.value));
    one.value = JSON.stringify(option.value);
    one.disabled = option.available === false;
    one.selected = JSON.stringify(option.value) === JSON.stringify(value);
    shown.append(one);
  }
  shown.addEventListener('change', () => pick(JSON.parse(shown.value)));
  return shown;
}

/** 下拉加「其他」：建议值里没有的写在后面那一行字里。 */
function combo(suggestions, value, put) {
  const box = element('span', 'inline');
  const other = !suggestions.includes(value);
  const shown = element('select', 'select');
  for (const suggestion of [...suggestions, null]) {
    const one = element('option', null, suggestion ?? say('plugins-page/other'));
    one.value = suggestion ?? '';
    one.selected = suggestion === null ? other : suggestion === value;
    shown.append(one);
  }
  const typed = element('input', 'input plug-text');
  typed.value = other ? value ?? '' : '';
  typed.hidden = !other;
  shown.addEventListener('change', () => {
    if (shown.value) return put({ value: shown.value });
    typed.hidden = false;
    typed.focus();
  });
  typed.addEventListener('change', () => put(typed.value.trim() ? { value: typed.value.trim() } : { unset: true }));
  box.append(shown, typed);
  return box;
}

/** 列表：一个一个小标签，后面一格加一个（shell/tags.js：回车、离开这一格都加，输入法选字的回车不算，加了当场画上）。 */
function list(items, put) {
  return tagInput({ items, save: (next) => put({ value: next }) });
}

/** 密钥：只说设没设，粘贴新的回车或离开就换上。 */
function secret(value, put) {
  // 默认圆点遮着、眼睛按钮看原文（施工 I-47）。
  const { box, input } = secretInput('input plug-text');
  input.placeholder = say(value ? 'plugins-page/secret-set' : 'plugins-page/secret-unset');
  input.addEventListener('change', () => {
    if (input.value.trim()) put({ value: input.value.trim() });
  });
  return box;
}
