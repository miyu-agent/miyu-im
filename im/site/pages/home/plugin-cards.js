// 首页上插件的卡片（施工 I-8 下，设计 01 第四节第 5 条；照样板的 .home-card）：声明了首页卡片、开着、在这个平台上能用、「放在首页」
// 没关的插件（2026-10-20 项目主人定：只在插件详情里开关，顺序照插件的编号），一个一张，放在「今天」那几个数下面。内容经插件登记的
// chat.home 取（plugins/widget-host.js）。卡片只造一次，首页重画时照旧放回去，不重取。
import { element, say } from '../../shell/i18n.js';
import { pluginIcon } from '../plugins/card.js';
import { mountWidget } from '../plugins/widget-host.js';
import { named } from '../venues/plugins.js';

/** 首页插件卡片那一格：交回 { box }（没有要放的插件时是空的，不占地方）。 */
export function pluginCards(request) {
  const box = element('div', 'home-grid plugin-cards');
  request('plugins.list').then(
    (answer) => {
      const shown = (answer.plugins || []).filter(
        (plugin) => plugin.ui?.home && plugin.on_home !== false && plugin.enabled && !plugin.unusable,
      );
      for (const plugin of shown) {
        const card = element('div', 'panel home-card');
        const title = element('h3');
        title.append(pluginIcon(plugin), document.createTextNode(named(plugin.name) || plugin.package), element('small', null, say('home/plugin')));
        const body = element('div', 'home-card-body');
        card.append(title, body);
        mountWidget(body, plugin.package, { place: 'home' }, {
          request,
          openPage: (id) => {
            location.hash = `plugins/${encodeURIComponent(plugin.package)}/page/${encodeURIComponent(id)}`;
          },
        });
        box.append(card);
      }
    },
    () => {},
  );
  return { box };
}
