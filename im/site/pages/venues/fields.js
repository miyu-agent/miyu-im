// 会话浮窗各页的格（施工 I-7 中一，名字、小字、默认照 docs/designs/01-接入通讯平台-配置项.md）：每一格写它是卡上的哪一格、什么控件、
// 哪几种卡有、要哪一格开着才画。form.js 照它画、存。
//
// - key：卡上的格；`rules.` 开头的是卡上 `rules` 里的规则键（宿主 card_edit 照项部分更新）。
// - control：select（下拉，`options` 交选项）、persona、preset（人格、预设的下拉，选项照宿主给的）、model（点开小窗选模型）、text、area、switch（`values` 是开、
//   关存成什么，不写是真假）、number（带 `unit`）、float（小数）、multi（几个小数一行，`keys` 各存各的）、seconds（规则里写成 `15s`
//   的时长，页面上填秒数）、rate（`N/Ms`：几条 / 几秒）、sleep（`HH:MM-HH:MM`）、tags（字的列表；`people` 的照号写、认得的写名字）、
//   percent（拖动条，百分之几，存成千分比）、reaction（贴表情：开关加点开小窗选）。`prefixed` 的标签只填了号的补成平台上的人的编号（`qq:10002`）。
// - clears：存这一格时一起去掉卡上旧写法的那几格（插嘴的四档 `chatty`：线路规程、抽样改写进 `rules` 以后不再用它）。
// - kinds：哪几种卡有（group、other_groups、private、other_privates），不写是都有。
// - dep：`{ key, on, unset? }`：那一格的值是 `on` 时才画（开关管着的几行关着时藏起来，设计第八节第 3 条）；那一格没写的照 `unset`，
//   不写 `unset` 的当是 `on`。switch 也认 `unset`：没写、也没继承到的照它画。
// - advanced：收进「高级」里（默认收起）。
// - needs：浮窗问到的那一样（`reactions`：平台能贴的表情；`presets`：预设）是空的、问不到的就不画（照回答里同名的那个列表）。
// - optionCaps：下拉的哪一项要平台做得到哪一样（施工 I-85 补，连接的 `caps`）：做不到的不列，卡上已经写了它的照旧列；会话默认值
//   那张不分平台，都列。

const GROUPS = ['group', 'other_groups'];

/** 主动回复判断开着才画（线路规程是 `chatty`）。 */
const JUDGE = { key: 'rules.discipline', on: 'chatty' };

/** 五项权重：相关性、意愿、社交适合度、时机、连续性。 */
const WEIGHTS = ['relevance', 'willingness', 'social', 'timing', 'continuity'].map((name) => `rules.chatty.${name}`);
const ALL_BUT_PRIVATE = ['group', 'other_groups', 'other_privates'];

/** 各页的格，照页的编号。 */
export const FIELDS = {
  persona: [
    { key: 'persona', control: 'persona', label: 'field/persona', hint: 'field/persona-hint' },
    { key: 'preset', control: 'preset', label: 'field/preset', needs: 'presets' },
    { key: 'extra_prompt', control: 'area', label: 'field/extra-prompt', hint: 'field/extra-prompt-hint', defaultsHint: 'field/extra-prompt-hint-defaults', wide: true },
    { group: 'field/models' },
    { key: 'model', control: 'model', label: 'field/model', use: 'chat' },
    // 判官没配模型的走旁路请求 `judge`（施工 I-42）：落到池的写池，池空了的照这张卡的模型（`orKey`），再照核心的对话模型。
    { key: 'rules.judge.model', control: 'model', label: 'field/judge-model', use: 'judge', orKey: 'model', kinds: GROUPS, dep: JUDGE },
    { group: 'field/context' },
    { key: 'recent', control: 'number', label: 'field/recent', unit: 'unit/messages', kinds: GROUPS, min: 1, max: 1000 },
    // 是否向 AI 显示号码（2026-10-20 项目主人验收：每种卡都有，原来只在群卡的触发方式页）。
    { key: 'show_ids', control: 'switch', label: 'field/show-ids' },
  ],
  trigger: [
    { key: 'keywords', control: 'tags', label: 'field/keywords', hint: 'field/keywords-hint', kinds: GROUPS },
    { key: 'rules.discipline', control: 'switch', values: ['chatty', 'when-called'], clears: ['chatty'], label: 'field/judge', hint: 'field/judge-hint', kinds: GROUPS },
    { key: 'rules.chatty.judge_called', control: 'switch', label: 'field/judge-called', hint: 'field/judge-called-hint', kinds: GROUPS, dep: JUDGE },
    { key: 'rules.judge.records', control: 'number', label: 'field/judge-records', unit: 'unit/messages', kinds: GROUPS, dep: JUDGE, min: 1, max: 100 },
    { key: 'rules.judge.persona', control: 'switch', label: 'field/judge-persona', hint: 'field/judge-persona-hint', kinds: GROUPS, dep: JUDGE },
    { key: 'rules.chatty.probability', control: 'percent', clears: ['chatty'], label: 'field/probability', hint: 'field/probability-hint', kinds: GROUPS, dep: JUDGE, max: 100 },
    { key: 'rules.chatty.skip_users', control: 'tags', people: true, prefixed: true, label: 'field/skip-users', hint: 'field/skip-users-hint', kinds: GROUPS, dep: JUDGE },
    { key: 'rules.chatty.skip_media', control: 'switch', label: 'field/skip-media', hint: 'field/skip-media-hint', kinds: GROUPS, dep: JUDGE },
    { key: 'managers', control: 'tags', people: true, label: 'field/managers', kinds: GROUPS },
    { key: 'rules.chatty.base', control: 'float', label: 'field/threshold', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.relevance', control: 'multi', keys: WEIGHTS, label: 'field/weights', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.restraint', control: 'switch', label: 'field/restraint', hint: 'field/restraint-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    // 跟进（施工 I-21）：她正在为这个人跑一轮时他又说了一条，过了判断并进那一轮。
    { key: 'rules.chatty.followup', control: 'switch', label: 'field/followup', hint: 'field/followup-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.continuation_window', control: 'seconds', label: 'field/continuation', hint: 'field/continuation-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.after_speaking_window', control: 'seconds', label: 'field/after-speaking', hint: 'field/after-speaking-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.dispatch.supersede_window', control: 'seconds', label: 'field/supersede', hint: 'field/supersede-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    // 违规判断的门槛（施工 I-16，照旧版设置页）：违规关键词在设置页「违规判断」。
    { key: 'rules.chatty.severity_min', control: 'number', label: 'field/severity-min', hint: 'field/severity-min-hint', kinds: GROUPS, advanced: true, min: 0, max: 10 },
  ],
  pace: [
    // 私聊白名单里的人（施工 I-7 中三下一）：忽略限流、忽略睡眠没写的是忽略（`unset`），关了才画限流、睡眠，写在卡上的 `rules` 里。
    { key: 'rules.ignore_rate', control: 'switch', unset: true, label: 'field/ignore-rate', kinds: ['private'] },
    { key: 'rules.rate', control: 'rate', label: 'field/rate', hint: 'field/rate-hint', kinds: ['private'], dep: { key: 'rules.ignore_rate', on: false, unset: true } },
    { key: 'rules.ignore_sleep', control: 'switch', unset: true, label: 'field/ignore-sleep', hint: 'field/ignore-sleep-hint', kinds: ['private'] },
    { key: 'rules.sleep', control: 'sleep', label: 'field/sleep', hint: 'field/sleep-hint', kinds: ['private'], dep: { key: 'rules.ignore_sleep', on: false, unset: true } },
    { key: 'rate', control: 'rate', label: 'field/rate', hint: 'field/rate-hint', kinds: ALL_BUT_PRIVATE },
    { key: 'sleep', control: 'sleep', label: 'field/sleep', hint: 'field/sleep-hint', kinds: ALL_BUT_PRIVATE },
    { key: 'rules.parallel', control: 'number', label: 'field/parallel', hint: 'field/parallel-hint', unit: 'unit/people', kinds: GROUPS, min: 0, max: 16 },
  ],
  send: [
    // 去掉末尾句号、一条分多条发送、打字速度（施工 I-49，项目主人 10-27：每张卡片单独设，原来在设置页「通用」里管所有会话）。
    { key: 'rules.outbound.strip_period', control: 'switch', label: 'field/strip-period', hint: 'field/strip-period-hint' },
    { key: 'rules.outbound.humanlike', control: 'switch', label: 'field/humanlike', hint: 'field/humanlike-hint' },
    { key: 'rules.outbound.typing_speed', control: 'number', label: 'field/typing-speed', hint: 'field/typing-speed-hint', unit: 'unit/chars-per-minute', min: 20, max: 600, dep: { key: 'rules.outbound.humanlike', on: true } },
    { key: 'final_only', control: 'switch', label: 'field/final-only', hint: 'field/final-only-hint' },
    { key: 'long_text', control: 'select', label: 'field/long-text', hint: 'field/long-text-hint', options: ['image', 'forward'], optionCaps: { forward: 'forward' }, optionText: 'long/' },
    { key: 'long_chars', control: 'number', label: 'field/long-chars', hint: 'field/long-chars-hint', unit: 'unit/chars', min: 0, max: 100000 },
    // 智能引用、智能艾特、贴表情（施工 I-7 中三下二）。贴表情要平台给了能贴的表情（`needs`）。
    { key: 'rules.outbound.quote', control: 'switch', label: 'field/quote', hint: 'field/quote-hint' },
    { key: 'rules.outbound.mention', control: 'switch', label: 'field/mention', hint: 'field/mention-hint', kinds: GROUPS },
    { key: 'rules.reaction', control: 'reaction', label: 'field/reaction', hint: 'field/reaction-hint', kinds: GROUPS, needs: 'reactions' },
    { key: 'rules.outbound.quote_after', control: 'number', label: 'field/quote-after', unit: 'unit/messages', advanced: true, min: 0, max: 1000, dep: { key: 'rules.outbound.quote', on: true } },
    { key: 'rules.outbound.mention_after', control: 'seconds', label: 'field/mention-after', kinds: GROUPS, advanced: true, min: 1, max: 86400, dep: { key: 'rules.outbound.mention', on: true } },
  ],
};

/** `one`（model.js 的一张卡）上 `field` 有没有。会话默认值（施工 I-7 下二，设置页）照群卡：群卡有的都有，私聊卡才有的没有。 */
export function has(field, one) {
  const kind = one.kind === 'defaults' ? 'group' : one.kind;
  return !field.kinds || field.kinds.includes(kind);
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

/** 卡上插嘴的旧写法（四档）各是千分之几（同宿主 `rules/cards.rs` 的 PROBABILITY）。 */
const CHATTY = { few: 20, some: 50, many: 120 };

/** 现在用着的：卡上写着的，没写的照下面几层。线路规程、抽样照卡上插嘴的旧写法（四档）算：`rules` 没写这两项、卡上写了插嘴的照它。 */
export function current(one, key) {
  const value = written(one, key);
  if (value !== undefined) return value;
  const old = one.set.chatty;
  if (old && key === 'rules.discipline') return old === 'never' ? 'when-called' : 'chatty';
  if (old && old !== 'never' && key === 'rules.chatty.probability') return CHATTY[old];
  return inherited(one, key).value;
}

/** 存 `key` 那一格成 `value`（null 是恢复默认）时交给 cards.save_one 的 `values`。 */
export function values(key, value) {
  if (key.startsWith('rules.')) return { rules: { [key.slice(6)]: value } };
  return { [key]: value };
}
