// 会话浮窗的「管理插件」（施工 I-7 中四，设计 01 第八节第 7 条）：在这个会话里用哪些插件。一个插件一行：名字、一句说明、开关。
//
// - 开没开照宿主的场所开关（`crate::plugins::Toggles`）算：这张卡上写了的照它；没写的照兜底卡（群照「陌生群」、私聊照「陌生私聊」，兜底卡自己
//   没有兜底）；都没写的照插件声明的默认。改了存进卡上的 `plugins`（和兜底一样的去掉，去光了整格去掉）。
// - 在这个平台上用不了的（读不成、只在别的平台上用、缺平台能力）灰着，写为什么，不能开关。
// - 点一个插件改它的设置、去它的详情页随插件页（I-8 下）。
import { element, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { cardOf } from './model.js';

/** 画 `one` 的这一页：`context` 是 { request, plugins（宿主认得的插件）, fallbacks（{group, private}：兜底卡上的开关） }。 */
export function drawPlugins(one, context) {
  const parts = [element('p', 'faint pane-hint', say('plugins/hint'))];
  const plugins = context.plugins || [];
  if (plugins.length === 0) parts.push(element('p', 'faint', say('plugins/none')));
  const fallback = one.id ? context.fallbacks?.[one.type] || {} : {};
  for (const plugin of plugins) parts.push(row(one, plugin, fallback, context.request));
  return parts;
}

/** 一个插件那一行。 */
function row(one, plugin, fallback, request) {
  const line = element('div', plugin.unusable ? 'row plugin off' : 'row plugin');
  const label = element('div', 'label');
  label.append(element('b', null, named(plugin.name) || plugin.package));
  const summary = named(plugin.summary);
  if (summary) label.append(element('small', null, summary));
  if (plugin.unusable) label.append(element('small', 'plugin-why', say(`plugins/unusable-${plugin.unusable}`)));
  const own = one.set.plugins || {};
  const inherited = fallback[plugin.package] ?? plugin.default ?? true;
  // 用不了的照关着画：开了也不会问它。
  const on = !plugin.unusable && (own[plugin.package] ?? inherited);
  const button = element('button', on ? 'switch on' : 'switch');
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
  line.append(label, button);
  return line;
}

/** 照页面的语言挑：没有的照英文、中文，再没有的照第一个。 */
function named(texts) {
  if (!texts) return '';
  const lang = document.documentElement.lang || 'en';
  return texts[lang] ?? texts.en ?? texts.zh ?? Object.values(texts)[0] ?? '';
}
