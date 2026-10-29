// 会话浮窗的「管理插件」（施工 I-7 中四，设计 01 第八节第 7 条）：在这个会话里用哪些插件。一个插件一行：名字、一句说明、开关。
//
// - 开没开照宿主的场所开关（`crate::plugins::Toggles`）算：这张卡上写了的照它；没写的照兜底卡（群照「陌生群」、私聊照「陌生私聊」，兜底卡自己
//   没有兜底）；都没写的照插件声明的默认。改了存进卡上的 `plugins`（和兜底一样的去掉，去光了整格去掉）。
// - 在这个平台上用不了的（读不成、只在别的平台上用、缺平台能力）灰着，写为什么，不能开关。
// - 名字后面一个小箭头去插件页上它的详情（施工 I-9 上）。插件页「用在哪」照同一套算法画开关（pluginSwitch）。
import { element, icon, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { cardOf } from './model.js';

/** 画 `one` 的这一页：`context` 是 { request, plugins（宿主认得的插件）, fallbacks（{group, private}：兜底卡上的开关） }。 */
export function drawPlugins(one, context) {
  const parts = [element('p', 'faint pane-hint', say('plugins/hint'))];
  const plugins = (context.plugins || []).filter((plugin) => fits(one, plugin));
  if (plugins.length === 0) parts.push(element('p', 'faint', say('plugins/none')));
  const fallback = one.id ? context.fallbacks?.[one.type] || {} : {};
  for (const plugin of plugins) parts.push(row(one, plugin, fallback, context.request));
  return parts;
}

/** 一个插件那一行。 */
function row(one, plugin, fallback, request) {
  const line = element('div', plugin.unusable ? 'row plugin off' : 'row plugin');
  const label = element('div', 'label');
  const title = element('b', null, named(plugin.name) || plugin.package);
  const detail = element('a', 'plugin-detail');
  detail.href = `#plugins/${encodeURIComponent(plugin.package)}`;
  detail.title = say('plugins/detail');
  detail.append(icon('i-up-right'));
  title.append(detail);
  label.append(title);
  const summary = named(plugin.summary);
  if (summary) label.append(element('small', null, summary));
  if (plugin.unusable) label.append(element('small', 'plugin-why', say(`plugins/unusable-${plugin.unusable}`)));
  line.append(label, pluginSwitch(one, plugin, fallback, request));
  return line;
}

/** 卡上的兜底：群照「陌生群」、私聊照「陌生私聊」卡上的插件开关（`all` 是 model.entries 的卡）。 */
export function fallbacksOf(all) {
  return {
    group: all.find((card) => card.kind === 'other_groups')?.set.plugins || {},
    private: all.find((card) => card.kind === 'other_privates')?.set.plugins || {},
  };
}

/** 插件 `plugin` 用不用得上卡 `one` 这种场所（施工 I-51）：声明了只在群（`group`）或只在私聊（`private`）用的，别的卡上不列。 */
export function fits(one, plugin) {
  const kind = one.type === 'private' ? 'private' : 'group';
  return !plugin.venues || plugin.venues.includes(kind);
}

/** 插件 `plugin` 在卡 `one` 上开没开（模块的说明）：`fallback` 是这张卡的兜底卡上的开关；用不了的算关着。 */
export function isOn(one, plugin, fallback) {
  const inherited = fallback[plugin.package] ?? plugin.default ?? true;
  return !plugin.unusable && ((one.set.plugins || {})[plugin.package] ?? inherited);
}

/** 插件 `plugin` 在卡 `one` 上的开关：点了存进卡上的 `plugins`（模块的说明），存不成的翻回去。 */
export function pluginSwitch(one, plugin, fallback, request) {
  const own = one.set.plugins || {};
  const inherited = fallback[plugin.package] ?? plugin.default ?? true;
  // 用不了的照关着画：开了也不会问它。
  const button = element('button', isOn(one, plugin, fallback) ? 'switch on' : 'switch');
  button.append(element('i'));
  button.disabled = Boolean(plugin.unusable);
  button.addEventListener('click', async () => {
    const next = !button.classList.contains('on');
    button.classList.toggle('on', next);
    const switches = { ...own };
    if (next === inherited) delete switches[plugin.package];
    else switches[plugin.package] = next;
    const values = { plugins: Object.keys(switches).length > 0 ? switches : null };
    try {
      const answer = await request('cards.save_one', { card: cardOf(one), values });
      if (answer.saved) return;
      toast(answer.message || say('failed/save'));
    } catch {
      toast(say('failed/save'));
    }
    button.classList.toggle('on', !next);
  });
  return button;
}

/** 照页面的语言挑：没有的照英文、中文，再没有的照第一个。 */
export function named(texts) {
  if (!texts) return '';
  const lang = document.documentElement.lang || 'en';
  return texts[lang] ?? texts.en ?? texts.zh ?? Object.values(texts)[0] ?? '';
}
