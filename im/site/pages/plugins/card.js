// 插件页上一个插件的卡片（施工 I-9 上，照样板的 .plug-card）：图标、名字、编号和版本、总开关，一句说明，几个小标签（挂在哪、有面板、
// 在这个平台上能不能用）。点卡片去它的详情。
import { element, icon, say } from '../../shell/i18n.js';
import { lucide } from '../../shell/lucide.js';
import { platform } from '../home/platforms.js';
import { named } from '../venues/plugins.js';
import { masterSwitch } from './switch.js';
import { goTo } from '../../shell/connection.js';

/** 插件 `plugin` 的卡片：`context` 是 { request, platform, reload }。 */
export function pluginCard(plugin, context) {
  const card = element('div', plugin.enabled ? 'plug-card' : 'plug-card off');
  const top = element('div', 'top');
  const grow = element('div', 'grow');
  grow.append(element('b', null, named(plugin.name) || plugin.package), element('div', 'faint', subtitle(plugin)));
  top.append(pluginIcon(plugin), grow, masterSwitch(plugin, context));
  card.append(top);
  const summary = named(plugin.summary);
  if (summary) card.append(element('p', null, summary));
  card.append(chips(plugin, context.platform));
  card.addEventListener('click', () => {
    goTo(`plugins/${encodeURIComponent(plugin.package)}`);
  });
  return card;
}

/** 编号、版本（出厂的写「出厂」，驱动带的写「内置」，施工 I-10 上）。 */
export function subtitle(plugin) {
  const parts = [plugin.package];
  if (plugin.builtin) parts.push(say('plugins-page/builtin'));
  else if (plugin.from === 'shipped') parts.push(say('plugins-page/shipped'));
  else if (plugin.version) parts.push(plugin.version);
  return parts.join(' · ');
}

/** 插件的图标：声明里写的 Lucide 名字，没写、认不出的画拼图。 */
export function pluginIcon(plugin) {
  const box = element('span', 'ico');
  box.append(plugin.icon ? lucide(plugin.icon, icon('i-puzzle')) : icon('i-puzzle'));
  return box;
}

/** 几个小标签：挂在哪几处、有面板、在宿主这个平台上能不能用（`host` 是平台的编号）。 */
export function chips(plugin, host) {
  const box = element('div', 'chips');
  for (const point of plugin.points || []) box.append(element('span', 'chip', say(`plugins-page/point-${point}`)));
  if (plugin.page) box.append(element('span', 'chip accent', say('plugins-page/has-panel')));
  box.append(element('span', plugin.unusable ? 'chip bad' : 'chip', usable(plugin, host)));
  return box;
}

/** 在平台 `host` 上能不能用，一句：能用的「QQ 可用」，只给几个平台的另说「只支持 QQ」，用不了的说为什么。 */
export function usable(plugin, host) {
  if (plugin.unusable) return say(`plugins/unusable-${plugin.unusable}`);
  const names = (plugin.platforms || []).map((id) => platform(id).name()).join(say('plugins-page/separator'));
  if (names) return say('plugins-page/only', { platforms: names });
  return say('plugins-page/usable', { platform: host ? platform(host).name() : '' });
}
