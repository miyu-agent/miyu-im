// 「模型」页的「模型池」（设计 09，施工 I-152；照网页软件 pools.js）：一个池一张卡片：头上池名（三个固定池写显示名、后面暗字接
// @flagship，2026-10-26 项目主人）、调用方式（固定、轮换、照先后）、「＋ 添加」（从还没进来的模型里勾好几个，接在最后）、删除（固定池
// 没有）；下面成员一行一个（序号、名字、哪一家，往上、往下、✕），最下面「子代理能选」开关。卡片下面「＋ 新建模型池」写名字回车建好
// （固定、没有成员）。改了当场存，写进系统配置。三个固定池排最前。
import { element, icon, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { lucide } from '../../shell/lucide.js';
import { duplicates, expectFor, filterModels, itemFor, keySegment, providerName, validId, writtenIn } from './config.js';

/** 三个固定池的先后：旗舰、日常、轻量。 */
export const FIXED_POOLS = ['flagship', 'normal', 'lite'];

/** 池的先后：固定池排最前，照 FIXED_POOLS；后面是人建的，照列表的先后（纯逻辑）。 */
export function poolOrder(pools) {
  const rank = (pool) => {
    const at = FIXED_POOLS.indexOf(pool.name);
    return at < 0 ? FIXED_POOLS.length : at;
  };
  return [...pools.filter((pool) => pool.fixed).sort((a, b) => rank(a) - rank(b)), ...pools.filter((pool) => !pool.fixed)];
}

/** 列表里第 `at` 个往前（-1）、往后（1）挪一格（纯逻辑）。 */
export function moveMember(list, at, step) {
  const to = at + step;
  if (to < 0 || to >= list.length) return list;
  const next = [...list];
  [next[at], next[to]] = [next[to], next[at]];
  return next;
}

/** 删一个池：这一层写了的 pools.<id>.* 各 unset（纯逻辑）。 */
export function poolRemoval(id, got) {
  const prefix = `pools.${keySegment(id)}.`;
  return Object.keys(got.items ?? {})
    .filter((key) => key.startsWith(prefix) && writtenIn(got.items[key]))
    .map((key) => ({ key, unset: true, expect: expectFor(got.items[key]) }));
}

/** 调用方式有哪几种：照 schema 的选项，认得的才列（纯逻辑）。 */
export function strategiesOf(item) {
  const known = ['pin', 'rotate', 'ordered'];
  const listed = (item?.options ?? []).map((one) => one.value).filter((value) => known.includes(value));
  return listed.length ? listed : ['pin', 'rotate'];
}

/** 模型池那一栏。`note(text)` 写一句存不成的。 */
export function drawPools(state, note) {
  const all = (state.models?.providers ?? []).flatMap((provider) =>
    (provider.models ?? []).map((model) => ({ ref: model.ref, model: model.model, name: model.facts?.name?.value ?? model.model, provider: providerName(provider) })),
  );
  const byRef = new Map(all.map((one) => [one.ref, one]));
  const field = (id, name) => itemFor(state.schema, state.got, `pools.<id>.${name}`, { id });
  const save = async (item, change) => note(await state.saveMany([{ key: item.key, ...change, expect: expectFor(item.entry) }]));

  const card = (pool) => {
    const members = field(pool.name, 'models');
    const strategy = field(pool.name, 'strategy');
    const now = members?.entry?.value ?? pool.models ?? [];
    const setMembers = (next) => members && save(members, { value: next });
    const head = element('div', 'pool-head');
    const title = element('strong', null, FIXED_POOLS.includes(pool.name) ? say(`models/pool-${pool.name}`) : `@${pool.name}`);
    if (FIXED_POOLS.includes(pool.name)) title.append(element('code', 'model-id', `@${pool.name}`));
    const seg = element('div', 'seg');
    for (const kind of strategiesOf(strategy)) {
      const button = element('button', pool.strategy === kind ? 'on' : null, say(`models/strategy-${kind}`));
      button.addEventListener('click', () => strategy && pool.strategy !== kind && save(strategy, { value: kind }));
      seg.append(button);
    }
    const add = element('button', 'btn');
    add.append(icon('i-plus'), element('span', null, say('models/pool-add')));
    add.addEventListener('click', () => pickModels(all.filter((one) => !now.includes(one.ref)), (picked) => setMembers([...now, ...picked])));
    head.append(title, element('span', 'grow'), seg, add);
    const removal = poolRemoval(pool.name, state.got);
    // 固定池没有删除，那一格留着空位（各张卡片的调用方式、「添加」上下对齐）。
    if (pool.fixed) head.append(element('span', 'pool-gap'));
    else if (removal.length) {
      const drop = element('button', 'icon-button');
      drop.title = say('models/pool-delete');
      drop.append(icon('i-x'));
      drop.addEventListener('click', async () => note(await state.saveMany(removal)));
      head.append(drop);
    }
    // 成员一行一个；名字、模型名都一样的（同一个模型）不算重名。配置里写着、认不出的照原样写。
    const known = now.map((ref) => byRef.get(ref) ?? { ref, name: ref, model: ref, provider: '' });
    const twice = duplicates([...new Map(known.map((one) => [`${one.name}\n${one.model}`, one.name])).values()]);
    const tool = (label, shown, act, off) => {
      const button = element('button', 'icon-button');
      button.title = say(label);
      button.append(shown);
      button.disabled = off;
      button.addEventListener('click', act);
      return button;
    };
    const list = element('div', 'pool-members');
    known.forEach((one, at) => {
      const row = element('div', 'pool-member');
      const name = element('span', 'model-line-name', one.name);
      if (twice.has(one.name)) name.append(element('code', 'model-id', one.model));
      row.append(
        element('span', 'faint', String(at + 1)),
        name,
        element('span', 'faint', one.provider),
        tool('models/pool-up', lucide('arrow-up', icon('i-down')), () => setMembers(moveMember(now, at, -1)), at === 0),
        tool('models/pool-down', lucide('arrow-down', icon('i-down')), () => setMembers(moveMember(now, at, 1)), at === now.length - 1),
        tool('models/pool-remove', icon('i-x'), () => setMembers(now.filter((_, other) => other !== at)), false),
      );
      list.append(row);
    });
    const box = element('div', 'model-main pool');
    box.append(head, known.length ? list : element('p', 'faint', say('models/pool-empty')));
    const sub = itemFor(state.schema, state.got, `pools.${keySegment(pool.name)}.subagent`, {}) ?? field(pool.name, 'subagent');
    if (sub) {
      const on = typeof sub.entry?.value === 'boolean' ? sub.entry.value : Boolean(pool.subagent);
      const toggle = element('button', on ? 'switch on' : 'switch');
      toggle.append(element('i'));
      toggle.addEventListener('click', () => save(sub, { value: !on }));
      const foot = element('div', 'model-row');
      foot.append(element('span', 'model-row-name', sub.name ?? say('models/pool-subagent')), toggle);
      box.append(foot);
    }
    return box;
  };

  const wrap = element('div', 'pools');
  wrap.append(...poolOrder(state.models?.pools ?? []).map(card));
  // 新建：一行「＋ 新建模型池」，点了换成写名字的框，回车建好，Esc 回去。
  const create = element('button', 'model-provider add');
  create.append(icon('i-plus'), element('span', null, say('models/new-pool')));
  create.addEventListener('click', () => {
    const box = element('div', 'model-main pool');
    const input = element('input', 'input persona-input');
    input.placeholder = say('models/pool-name-hint');
    const hint = element('span', 'faint', say('models/pool-new-hint'));
    box.append(input, hint);
    const cancel = () => box.replaceWith(create);
    input.addEventListener('keydown', async (event) => {
      if (event.key === 'Escape') return cancel();
      if (event.key !== 'Enter' || event.isComposing) return;
      const id = input.value.trim();
      const fail = (why) => {
        hint.textContent = why;
        hint.className = 'persona-note';
      };
      if (!validId(id)) return fail(say('models/form-bad-id'));
      if ((state.models?.pools ?? []).some((pool) => pool.name === id)) return fail(say('models/form-taken-id', { id }));
      const base = `pools.${keySegment(id)}`;
      const why = await state.saveMany([
        { key: `${base}.strategy`, value: 'pin', expect: {} },
        { key: `${base}.models`, value: [], expect: {} },
      ]);
      if (why) fail(why);
    });
    create.replaceWith(box);
    input.focus();
  });
  wrap.append(create);
  return wrap;
}

/** 勾好几个模型的小浮窗：搜索框，一行一个（名字、哪一家），「添加 N 个」照勾的先后交回。 */
function pickModels(rows, done) {
  const dialog = openDialog(say('models/pool-add'));
  dialog.body.classList.add('persona-body');
  const picked = [];
  const find = element('input', 'input persona-input');
  find.type = 'search';
  find.placeholder = say('models/search');
  const list = element('div', 'model-lines');
  const go = element('button', 'btn primary');
  const sync = () => {
    go.textContent = say('models/picker-add', { count: picked.length });
    go.disabled = !picked.length;
  };
  const shown = rows.map((one) => {
    const node = element('button', 'model-line');
    node.append(element('span', 'model-line-name', one.name), element('span', 'faint', one.provider));
    node.addEventListener('click', () => {
      const at = picked.indexOf(one.ref);
      if (at >= 0) picked.splice(at, 1);
      else picked.push(one.ref);
      node.classList.toggle('on', at < 0);
      sync();
    });
    list.append(node);
    return { ...one, node };
  });
  const empty = element('p', 'faint', say(rows.length ? 'models/no-match' : 'models/picker-none'));
  empty.hidden = rows.length > 0;
  find.addEventListener('input', () => {
    const hits = new Set(filterModels(shown, find.value));
    for (const one of shown) one.node.hidden = !hits.has(one);
    empty.hidden = hits.size > 0;
  });
  go.addEventListener('click', () => {
    dialog.close();
    done([...picked]);
  });
  const cancel = element('button', 'btn', say('dialog/cancel'));
  cancel.addEventListener('click', () => dialog.close());
  sync();
  dialog.body.append(find, list, empty);
  dialog.foot.append(element('span', 'grow'), cancel, go);
  find.focus();
}
