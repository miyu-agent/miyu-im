// 使用限制页（设计 08 第二节第 6 条，施工 I-136）：每个人最近 5 小时、7 天、30 天最多用多少 token，一个人在所有群和私聊里合起来算。
// 三段：默认的三条线（对所有人）、单独设的人（整组替换默认的，三格都空是不限）、最近 30 天用过的人（照 5 小时的用量排，满了的标红）。
// 会话自己的上限在会话卡片「节奏」页，不在这里。
//
// - 读写经宿主的 limits.get、limits.set（整份替换 im 配置目录里的 limits.toml）；改了当场存，存不成的提示一句、重读一遍。
// - 一格空着是不限；填的是 1 到一万亿的整数，别的提示一句不存。格后面写单位 token（施工 I-168）。
// - 一段一张共用的卡片（shell/sect.js，设计 11 G1）；加人先选平台再填号码，照用过的人提示（shell/person-pick.js，设计 11 G4）；
//   30 天用量能搜、能照三条线的用量排、能只看到了上限的（shell/listing.js，设计 11 G2）。
import { element, icon, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { listing } from '../../shell/listing.js';
import { personPick } from '../../shell/person-pick.js';
import { sect } from '../../shell/sect.js';
import { chosenOf } from '../../shell/connection.js';
import { short } from '../stats/stats.js';

/** 三条线的键，照 5 小时、7 天、30 天。 */
const KEYS = ['limit_5h', 'limit_week', 'limit_month'];

/** 一条线最多写多大。 */
const CAP = 1e12;

/** 在 `view` 里起使用限制页：`request` 调宿主的方法。交回 update()、event()、dispose()。 */
export function mountLimits(view, { request }) {
  let got = null;
  // 外壳问到的连接（加人时选平台），30 天用量的列表头搜了什么、怎么排（重画时照旧）。
  let connections = [];
  const usageState = {};
  const load = async () => {
    try {
      got = await request('limits.get');
    } catch {
      view.replaceChildren(head(), element('p', 'faint', say('limits-page/failed')));
      return;
    }
    draw();
  };
  const save = async () => {
    let saved = false;
    try {
      saved = (await request('limits.set', { default: got.default, people: got.people })).saved;
    } catch {
      saved = false;
    }
    if (!saved) toast(say('failed/save'));
    load();
  };
  const draw = () => {
    if (!got) return;
    const names = new Map((got.usage || []).map((one) => [one.id, one.name]));
    const platforms = [...new Set(connections.map((one) => one.platform))];
    const chosen = chosenOf(connections)?.platform;
    view.replaceChildren(head(), defaults(got, save), people(got, { names, platforms, chosen }, save), usage(got.usage || [], usageState));
  };
  load();
  return {
    update(now) {
      connections = now || [];
      draw();
    },
    event() {},
    dispose() {},
  };
}

function head() {
  const shown = element('div', 'view-head');
  shown.append(element('h1', null, say('nav/limits')));
  return shown;
}

/** 一段：一张卡片（shell/sect.js），组名、小字、下面的几行。 */
function section(title, hint, rows) {
  const card = sect(say(title), hint ? say(hint) : null);
  card.body.append(...rows);
  return card.box;
}

/** 一行：名字、小字、控件。 */
function row(label, hint, ...controls) {
  const line = element('div', 'row');
  const text = element('div', 'label', label);
  if (hint) text.append(element('small', null, hint));
  line.append(text, ...controls);
  return line;
}

/** 一条线的格：`lines` 上 `key` 那一条，改了写回去、交给 `save`。空的是不限；写错的提示一句，照原样画回去。`unit` 的后面写 token
 *  （单独设的人那一行上面有列名，不写）。 */
function cap(lines, key, save, unit = true) {
  const input = element('input', 'input num wide-num');
  input.inputMode = 'numeric';
  input.placeholder = say('limits-page/unlimited');
  input.value = grouped(lines[key]);
  input.addEventListener('change', () => {
    const typed = input.value.trim().replace(/[,_\s]/g, '');
    if (typed === '') {
      lines[key] = null;
      save();
      return;
    }
    const number = Number(typed);
    if (!Number.isInteger(number) || number < 1 || number > CAP) {
      toast(say('failed/invalid'));
      input.value = grouped(lines[key]);
      return;
    }
    lines[key] = number;
    save();
  });
  if (!unit) return input;
  const box = element('span', 'inline');
  box.append(input, document.createTextNode(say('unit/tokens')));
  return box;
}

/** 格里的数照三位一撇写（2,000,000），存的时候去掉；没有的是空的。 */
function grouped(value) {
  return value == null ? '' : Number(value).toLocaleString('en-US');
}

/** 单独设的人那一行的一格：窄的屏上列名藏了，格上面另带一行小字说是哪一条线。 */
function labeled(key, input) {
  const box = element('label', 'limits-cap');
  box.append(element('span', 'limits-cap-label', say(`limits-page/${key}`)), input);
  return box;
}

/** 默认的三条线：一条一行。 */
function defaults(got, save) {
  got.default = got.default || {};
  const rows = KEYS.map((key) => row(say(`limits-page/${key}`), null, cap(got.default, key, save)));
  return section('limits-page/default', 'limits-page/default-hint', rows);
}

/** 单独设的人：一人一行（名字或号、三格、去掉），最后一行加一个：先选平台再填号码（`platforms` 是挂着的平台，`chosen` 是首页选中的
 *  连接的），照用过的人提示。 */
function people(got, { names, platforms, chosen }, save) {
  got.people = got.people || [];
  const rows = got.people.map((one, at) => {
    const remove = element('button', 'icon-button');
    remove.title = say('limits-page/remove');
    remove.append(icon('i-x'));
    remove.addEventListener('click', () => {
      got.people.splice(at, 1);
      save();
    });
    const caps = element('div', 'limits-caps');
    caps.append(...KEYS.map((key) => labeled(key, cap(one, key, save, false))));
    const name = names.get(one.id);
    return row(name || one.id, name ? one.id : null, caps, remove);
  });
  const pick = personPick({
    platforms,
    chosen,
    people: [...names].map(([id, name]) => ({ id, name })),
    label: say('limits-page/add'),
    pick: (typed) => {
      if (!/^[a-z][a-z0-9_-]*:\S+$/.test(typed) || got.people.some((one) => one.id === typed)) {
        toast(say('failed/invalid'));
        return;
      }
      got.people.push({ id: typed });
      save();
    },
  });
  const adding = row(say('limits-page/add-label'), say('limits-page/add-hint'), pick);
  adding.classList.add('limits-add');
  rows.push(adding);
  // 列名对着下面的三格；最后垫一个看不见的按钮，和「去掉」一样宽。
  const columns = element('div', 'limits-caps');
  columns.append(...KEYS.map((key) => element('span', 'limits-col', say(`limits-page/${key}`))));
  const spacer = element('button', 'icon-button limits-spacer');
  spacer.append(icon('i-x'));
  spacer.tabIndex = -1;
  const header = row('', null, columns, spacer);
  header.classList.add('limits-header');
  return section('limits-page/people', 'limits-page/people-hint', got.people.length ? [header, ...rows] : rows);
}

/** 最近 30 天用过的人：名字、三条线各用了多少（有线的写「用了 / 上限」），满了的标红。能搜、能照三条线的用量排、能只看到了上限的；
 *  `state` 记着搜了什么、怎么排。 */
function usage(list, state) {
  if (list.length === 0) {
    return section('limits-page/usage', 'limits-page/usage-hint', [element('p', 'faint', say('limits-page/nobody'))]);
  }
  const header = element('div', 'limits-usage-row limits-usage-head');
  header.append(element('span', 'th', say('limits-page/who')));
  for (const key of KEYS) header.append(element('span', 'th num', say(`limits-page/${key}`)));
  const items = list.map((one) => {
    const line = element('div', 'limits-usage-row');
    const who = element('span', one.full ? 'name full' : 'name', one.name ? `${one.name} · ${one.id}` : one.id);
    who.title = who.textContent;
    line.append(who);
    one.used.forEach((used, at) => {
      const limit = one.caps[at];
      const reached = limit != null && used >= limit;
      const text = limit == null ? short(used) : `${short(used)} / ${short(limit)}`;
      line.append(element('span', reached ? 'num full' : 'num', text));
    });
    const numbers = Object.fromEntries(KEYS.map((key, at) => [key, one.used[at] ?? 0]));
    return { node: line, text: who.textContent, tags: { full: one.full ? 'full' : 'under' }, numbers };
  });
  const table = listing(items, {
    search: say('listing/search'),
    sorts: KEYS.map((key) => [key, say('limits-page/sort', { line: say(`limits-page/${key}`) })]),
    filters: [{ key: 'full', all: say('limits-page/everyone'), options: [['full', say('limits-page/only-full')]] }],
    state,
    header,
  });
  table.classList.add('limits-usage');
  return section('limits-page/usage', 'limits-page/usage-hint', [table]);
}
