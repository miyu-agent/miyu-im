// 会话浮窗里点开小窗选的两格（2026-10-20 项目主人验收：原生下拉里模型多了找不到）：模型（照供应商分组、能搜）、贴表情（表情格子、
// 能搜、能直接填数字编号）。小窗是 shell/picker.js。
import { element, say } from '../../shell/i18n.js';
import { openPicker, pickButton } from '../../shell/picker.js';
import { current } from './fields.js';

/** 选模型：`models` 是宿主 `models` 的回答（{models: [{ref, state?}], pools, uses}），`use` 是继承到底时核心那一项（chat）。 */
export function modelControl(field, mine, fallback, models, save) {
  const known = models || { models: [], pools: [], uses: {} };
  const used = fallback ?? known.uses?.[field.use] ?? null;
  const inherit = say('field/inherit', { value: used ?? '—' });
  const button = pickButton(mine ?? inherit, () =>
    openPicker({
      title: say('picker/model-title'),
      placeholder: say('picker/model-search'),
      groups: modelGroups(known, mine),
      first: { value: null, text: inherit },
      current: mine ?? null,
      pick: (value) => {
        button.setText(value ?? inherit);
        mine = value ?? undefined;
        save(value);
      },
    }),
  );
  return button;
}

/** 模型照供应商（引用 `<供应商>/<模型>` 的前一段）分组，池另一组；写着的不在里面的另列一组。 */
function modelGroups(known, mine) {
  const groups = new Map();
  for (const model of known.models || []) {
    const at = model.ref.indexOf('/');
    const provider = at > 0 ? model.ref.slice(0, at) : model.ref;
    if (!groups.has(provider)) groups.set(provider, []);
    groups.get(provider).push({
      value: model.ref,
      text: at > 0 ? model.ref.slice(at + 1) : model.ref,
      sub: model.state ? say('field/model-unusable') : undefined,
    });
  }
  const all = [...groups].map(([name, items]) => ({ name, items }));
  const pools = (known.pools || []).map((pool) => ({ value: `@${pool}`, text: `@${pool}` }));
  if (pools.length > 0) all.push({ name: say('picker/pools'), items: pools });
  const listed = all.some((group) => group.items.some((item) => item.value === mine));
  if (mine && !listed) all.push({ name: say('picker/other'), items: [{ value: mine, text: mine }] });
  return all;
}

/** 贴表情（施工 I-7 中三下二）：开关加选哪个。规则写表情的编号，`off` 是不贴；没写的照桥的出厂（`known.default`）。关掉存
 *  `off`，打开存选着的那一个；选哪个点开表情小窗，能搜名字、能直接填数字编号。 */
export function reactionControl(field, one, known, save) {
  const box = element('span', 'inline');
  const now = current(one, field.key);
  const on = now !== 'off';
  let chosen = on && now ? String(now) : String(known.default);
  const button = element('button', on ? 'switch on' : 'switch');
  button.append(element('i'));
  const nameOf = (id) => {
    const found = known.reactions.find((each) => String(each.id) === id);
    if (!found) return say('picker/reaction-id', { id });
    return found.glyph ? `${found.glyph} ${found.name}` : found.name;
  };
  const pick = pickButton(nameOf(chosen), () =>
    openPicker({
      title: say('picker/reaction-title'),
      placeholder: say('picker/reaction-search'),
      grid: true,
      groups: [{ items: known.reactions.map((each) => ({ value: String(each.id), text: each.name, sub: String(each.id), glyph: each.glyph })) }],
      current: chosen,
      custom: (typed) => (/^\d{1,6}$/.test(typed) ? typed : null),
      customText: (id) => say('picker/use-id', { id }),
      pick: (value) => {
        if (value == null) return;
        chosen = String(value);
        pick.setText(nameOf(chosen));
        save(chosen);
      },
    }),
  );
  pick.hidden = !on;
  button.addEventListener('click', () => {
    const next = !button.classList.contains('on');
    button.classList.toggle('on', next);
    pick.hidden = !next;
    save(next ? chosen : 'off');
  });
  box.append(pick, button);
  return box;
}
