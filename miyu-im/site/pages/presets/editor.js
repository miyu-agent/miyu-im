// 预设的详情（设计 09，施工 I-147；照网页软件的 preset-editor.js）：浮窗里名字、简单描述、图标、无人值守步数、功能开关树，都是当场存
// （preset.set 只带改的那一项，不带 expect），没有「保存」，关浮窗不问。
//
// - 名字、简单描述：回车、离开框就存；描述清空发空字（2026-10-23 项目主人定预设也要描述）。
// - 图标：现在的图标加 ▼，点开一排：搜索框（照整套 Lucide 名字找）、「不用图标」、图标格子（出厂先列网页软件 preset_icons 那一组）。
// - 无人值守步数（核心 AE-1，2026-10-30 项目主人：一格数字框）：回应里有这一格才画；空着照出厂的 500，只认正整数，清空发 unset。
// - 功能：搜索框（2026-10-27 项目主人：预设列表没法搜）按组名、功能名、工具名筛，输入法选字时不筛；树在 tree.js。
// - 删除、恢复出厂照核心给的 remove，先弹窗问；预设不带默认人格，人格、预设互不引用（2026-10-08）。
import { element, icon, say } from '../../shell/i18n.js';
import { confirmFirst, openDialog, toast } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { iconNames, lucide } from '../../shell/lucide.js';
import { fallbackSections, featureTree, filterTree } from './tree.js';

/** 选图标时先列的那一组（同网页软件设置项 preset_icons 的出厂值）。 */
export const PRESET_ICONS = [
  'box', 'boxes', 'package', 'layers', 'wrench', 'hammer', 'code-xml', 'square-terminal', 'bot', 'brain', 'sparkles', 'zap', 'rocket',
  'shield', 'lock', 'eye', 'book-open', 'pencil', 'pen-tool', 'palette', 'music', 'gamepad-2', 'heart', 'star', 'sun', 'moon', 'coffee',
  'briefcase', 'graduation-cap', 'flask-conical', 'globe', 'message-circle', 'users', 'user-round', 'house', 'folder', 'file-text',
  'search', 'settings', 'cpu', 'database', 'cloud', 'leaf', 'flame', 'feather', 'gem', 'crown', 'anchor', 'compass', 'map', 'camera',
  'image', 'film', 'headphones', 'mic', 'calendar', 'clock',
];

/** 开预设 `id` 的详情：`name` 是列表里写的；存了调 `hooks.saved()`，删了调 `hooks.removed(remains)`。 */
export function openPreset(request, id, name, hooks) {
  let picking = false;
  let query = '';
  let findFocused = false;
  let got = null;
  const dialog = openDialog(name || id);
  dialog.body.classList.add('persona-body');
  const message = element('p', 'persona-note');
  message.hidden = true;
  const note = (text) => {
    message.hidden = !text;
    message.textContent = text;
  };

  const set = async (changes) => {
    try {
      got = await core(request, 'preset.set', { preset: id, changes });
      note('');
      draw();
      hooks.saved();
      return true;
    } catch (error) {
      note(refusalText(error));
      return false;
    }
  };

  const draw = () => {
    const label = typeof got.name === 'string' ? got.name : '';
    dialog.setTitle(label || id);
    const named = textBox(label, '', (text) => text && text !== label && set([{ key: 'preset.name', value: text }]));
    const said = typeof got.summary === 'string' ? got.summary : '';
    const summary = textBox(said, say('personas/summary-hint'), (text) => {
      if (text.trim() !== said.trim()) set([{ key: 'preset.summary', value: text.trim() }]);
    });
    const sections = Array.isArray(got.sections)
      ? got.sections
      : fallbackSections(Array.isArray(got.features) ? got.features : [], say('presets/features'));
    const rows = [
      field('personas/name', named),
      field('personas/summary', summary),
      field('presets/icon', iconRow(label), 'div'),
    ];
    if ('unattended_steps' in got) rows.push(stepsRow(got.unattended_steps));
    rows.push(field('presets/features', sections.length ? features(sections) : element('p', 'faint', say('presets/no-features')), 'div'));
    rows.push(message);
    dialog.body.replaceChildren(...rows);
    dialog.foot.replaceChildren(...(got.remove ? [removeButton(label || id)] : []));
    dialog.foot.hidden = !got.remove;
  };

  const features = (sections) => {
    const box = element('div');
    const tree = () => {
      const shown = filterTree(sections, query);
      return shown.length ? featureTree(shown, set, got.unlisted) : element('p', 'faint', say('presets/no-match'));
    };
    box.append(tree());
    const find = element('input', 'input persona-input');
    find.type = 'search';
    find.value = query;
    find.placeholder = say('presets/find');
    const refilter = () => {
      query = find.value;
      box.replaceChildren(tree());
    };
    find.addEventListener('input', (event) => {
      if (!event.isComposing) refilter();
    });
    find.addEventListener('compositionend', refilter);
    find.addEventListener('focus', () => {
      findFocused = true;
    });
    find.addEventListener('blur', () => {
      findFocused = false;
    });
    // 存了一项重画时照旧留着搜的字和焦点。
    if (findFocused) queueMicrotask(() => find.focus());
    const all = element('div', 'preset-features');
    all.append(find, box);
    return all;
  };

  const iconRow = (label) => {
    const now = typeof got.icon === 'string' ? got.icon : null;
    const current = element('button', picking ? 'preset-icon-now open' : 'preset-icon-now');
    current.title = say('presets/icon');
    current.append(now ? lucide(now, element('span', null, [...label][0] ?? '')) : element('span', null, [...label][0] ?? ''), icon('i-down'));
    current.addEventListener('click', () => {
      picking = !picking;
      draw();
    });
    if (!picking) return current;
    const pick = (chosen) => {
      picking = false;
      set([chosen ? { key: 'preset.icon', value: chosen } : { key: 'preset.icon', unset: true }]);
    };
    const cells = element('div', 'preset-icon-grid');
    const fill = (names) => {
      cells.replaceChildren(
        ...(names.length
          ? names.map((one) => {
              const cell = element('button', one === now ? 'preset-icon on' : 'preset-icon');
              cell.title = one;
              cell.append(lucide(one, icon('i-puzzle')));
              cell.addEventListener('click', () => pick(one));
              return cell;
            })
          : [element('p', 'faint', say('presets/icon-empty'))]),
      );
    };
    const find = element('input', 'input persona-input');
    find.placeholder = say('presets/icon-search');
    find.addEventListener('input', async () => {
      const wanted = find.value.trim().toLowerCase();
      if (!wanted) return fill(PRESET_ICONS);
      const names = await iconNames();
      fill(names.filter((one) => one.includes(wanted)).slice(0, PRESET_ICONS.length));
    });
    const none = element('button', 'btn ghost', say('presets/icon-none'));
    none.addEventListener('click', () => pick(null));
    const tools = element('div', 'preset-icon-tools');
    tools.append(find, none);
    fill(PRESET_ICONS);
    const box = element('div', 'preset-icon-pick');
    box.append(current, tools, cells);
    return box;
  };

  const stepsRow = (now) => {
    const was = typeof now === 'number' ? String(now) : '';
    const bad = element('p', 'persona-note', say('presets/unattended-bad'));
    bad.hidden = true;
    const box = textBox(was, say('presets/unattended-hint'), (text) => {
      const typed = text.trim();
      if (typed === was) return;
      bad.hidden = true;
      if (!typed) return set([{ key: 'limits.unattended_steps', unset: true }]);
      const steps = parseSteps(typed);
      if (steps === null) {
        box.value = was;
        bad.hidden = false;
        return;
      }
      set([{ key: 'limits.unattended_steps', value: steps }]);
    });
    box.inputMode = 'numeric';
    box.title = say('presets/unattended-tip');
    const both = element('div');
    both.append(box, bad);
    const row = field('presets/unattended', both, 'div');
    row.title = say('presets/unattended-tip');
    return row;
  };

  const removeButton = (shown) => {
    const kind = got.remove;
    const button = element('button', 'btn ghost danger', say(`personas/${kind}`));
    button.addEventListener('click', async () => {
      if (!(await confirmFirst(say(`personas/${kind}`), say(`personas/${kind}-ask`, { name: shown }), say(`personas/${kind}`)))) return;
      try {
        const done = await core(request, 'preset.delete', { preset: id });
        toast(say(done.remains ? 'personas/restored' : 'personas/deleted'));
        hooks.removed(Boolean(done.remains));
        if (done.remains) load();
        else dialog.close();
      } catch (error) {
        note(refusalText(error));
      }
    });
    return button;
  };

  const load = async () => {
    try {
      got = await core(request, 'preset.get', { preset: id });
    } catch (error) {
      dialog.body.replaceChildren(element('p', 'persona-note', refusalText(error)));
      return;
    }
    draw();
  };
  load();
}

/** 一格字：回车、离开框调 `commit(text)`。 */
function textBox(value, hint, commit) {
  const box = element('input', 'input persona-input');
  box.value = value;
  box.placeholder = hint;
  let last = value;
  const done = () => {
    if (box.value === last) return;
    last = box.value;
    commit(box.value);
  };
  box.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.isComposing) done();
  });
  box.addEventListener('blur', done);
  return box;
}

/** 一格：上面名字、下面控件。 */
function field(label, control, tag = 'label') {
  const box = element(tag, 'persona-field');
  box.append(element('span', 'persona-label', say(label)), control);
  return box;
}

/** 无人值守步数：正整数交回数，别的交回 null（纯逻辑）。 */
export function parseSteps(typed) {
  if (!/^\d+$/.test(typed)) return null;
  const steps = Number(typed);
  return Number.isSafeInteger(steps) && steps >= 1 ? steps : null;
}
