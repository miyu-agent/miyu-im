// 会话浮窗各页的格（施工 I-7 中一，名字、小字、默认照 docs/designs/01-接入通讯平台-配置项.md）：每一格写它是卡上的哪一格、什么控件、
// 哪几种卡有、要哪一格开着才画。form.js 照它画、存。
//
// - key：卡上的格；`rules.` 开头的是卡上 `rules` 里的规则键（宿主 card_edit 照项部分更新）。
// - control：select（下拉，`options` 交选项）、persona、model（人格、模型的下拉，选项照宿主给的）、text、area、switch、number（带
//   `unit`）、seconds（规则里写成 `15s` 的时长，页面上填秒数）、rate（`N/Ms`：几条 / 几秒）、sleep（`HH:MM-HH:MM`）。
// - kinds：哪几种卡有（group、other_groups、private、other_privates），不写是都有。
// - dep：`{ key, on }`：那一格的值是 `on` 时才画（开关管着的几行关着时藏起来，设计第八节第 3 条）。
// - advanced：收进「高级」里（默认收起）。

const GROUPS = ['group', 'other_groups'];
const ALL_BUT_PRIVATE = ['group', 'other_groups', 'other_privates'];

/** 各页的格，照页的编号。 */
export const FIELDS = {
  persona: [
    { key: 'persona', control: 'persona', label: 'field/persona', hint: 'field/persona-hint' },
    { key: 'extra_prompt', control: 'area', label: 'field/extra-prompt', hint: 'field/extra-prompt-hint', wide: true },
    { group: 'field/models' },
    { key: 'model', control: 'model', label: 'field/model', use: 'chat' },
    { key: 'rules.judge.model', control: 'model', label: 'field/judge-model', use: 'chat', kinds: GROUPS, dep: { key: 'rules.discipline', on: 'chatty' } },
    { group: 'field/context', kinds: GROUPS },
    { key: 'recent', control: 'number', label: 'field/recent', unit: 'unit/messages', kinds: GROUPS, min: 1, max: 1000 },
  ],
  pace: [
    { key: 'rate', control: 'rate', label: 'field/rate', hint: 'field/rate-hint', kinds: ALL_BUT_PRIVATE },
    { key: 'sleep', control: 'sleep', label: 'field/sleep', hint: 'field/sleep-hint', kinds: GROUPS },
    { key: 'rules.parallel', control: 'number', label: 'field/parallel', hint: 'field/parallel-hint', unit: 'unit/people', kinds: GROUPS, min: 1, max: 100 },
  ],
  send: [
    { key: 'final_only', control: 'switch', label: 'field/final-only', hint: 'field/final-only-hint' },
    { key: 'long_text', control: 'select', label: 'field/long-text', hint: 'field/long-text-hint', options: ['split', 'image', 'forward'], optionText: 'long/' },
    { key: 'long_chars', control: 'number', label: 'field/long-chars', hint: 'field/long-chars-hint', unit: 'unit/chars', min: 0, max: 100000 },
    { key: 'rules.outbound.quote_after', control: 'number', label: 'field/quote-after', unit: 'unit/messages', advanced: true, min: 0, max: 1000 },
    { key: 'rules.outbound.mention_after', control: 'seconds', label: 'field/mention-after', kinds: GROUPS, advanced: true, min: 1, max: 86400 },
  ],
};

/** `one`（model.js 的一张卡）上 `field` 有没有。 */
export function has(field, one) {
  return !field.kinds || field.kinds.includes(one.kind);
}

/** 卡上写着的值：没写的是 undefined。 */
export function written(one, key) {
  if (key.startsWith('rules.')) return one.set.rules?.[key.slice(6)];
  return one.set[key];
}

/** 卡下面几层的值和来处 `{value, from}`：卡片默认值里没有的是空的。 */
export function inherited(one, key) {
  if (key.startsWith('rules.')) return one.defaults.rules?.[key.slice(6)] || { value: null, from: null };
  return one.defaults[key] || { value: null, from: null };
}

/** 现在用着的：卡上写着的，没写的照下面几层。 */
export function current(one, key) {
  const value = written(one, key);
  return value === undefined ? inherited(one, key).value : value;
}

/** 存 `key` 那一格成 `value`（null 是恢复默认）时交给 cards.save_one 的 `values`。 */
export function values(key, value) {
  if (key.startsWith('rules.')) return { rules: { [key.slice(6)]: value } };
  return { [key]: value };
}
