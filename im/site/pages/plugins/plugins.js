// 插件页（施工 I-9 上，设计 01 第四节、第八节第 4 条；照样板 d-im.html 的插件页）：装了的 im 插件一张一张排开（card.js），点开是
// 它的详情（detail.js，网址 #plugins/<编号>）。
//
// - 插件照宿主的 plugins.list 画（im 认得的并上核心说的版本、开关、挂在哪）；「用在哪」、开着几个会话照 cards.get 的卡片算。
// - 宿主推 cards（卡片变了）再读一遍；开关了插件也再读一遍。开着面板（#plugins/<编号>/panel，施工 I-9 下，panel.js）、整页
//   （#plugins/<编号>/page/<整页的编号>，施工 I-8 下，照界面块画）的不重画：它们有自己的状态。
import { element, icon, say } from '../../shell/i18n.js';
import { pluginCard } from './card.js';
import { drawDetail } from './detail.js';
import { drawPanel } from './panel.js';
import { mountWidget } from './widget-host.js';
import { named } from '../venues/plugins.js';

/** 在 `view` 里起插件页：`request` 调宿主的方法，`path` 是网址里 #plugins/ 后面的那一段（插件的编号，没有的是列表）。 */
export function mountPlugins(view, { request, path }) {
  let disposed = false;
  const [wanted, sub, pageId] = path.split('/');
  // 开着的面板：只画一次，页面换走时收掉。
  let panel = null;
  const load = async () => {
    let listed;
    let cards;
    let status;
    try {
      [listed, cards, status] = await Promise.all([request('plugins.list'), request('cards.get'), request('status')]);
    } catch {
      if (!disposed) view.replaceChildren(head(null), element('p', 'faint', say('plugins-page/failed')));
      return;
    }
    if (disposed) return;
    const plugins = listed.plugins || [];
    const context = { request, platform: status.platform || '', cards, reload: load };
    if (wanted) {
      const plugin = plugins.find((one) => one.package === wanted);
      if (!plugin) {
        view.replaceChildren(back(), element('p', 'faint', say('plugins-page/gone')));
      } else if (sub === 'panel') {
        if (panel) return;
        panel = drawPanel(plugin, context);
        view.classList.add('panel-view');
        view.replaceChildren(...panel.parts);
      } else if (sub === 'page') {
        if (panel) return;
        panel = { close() {} };
        view.replaceChildren(...widgetPage(plugin, pageId, request));
      } else {
        view.replaceChildren(...drawDetail(plugin, context));
      }
      return;
    }
    const grid = element('div', 'plug-grid');
    grid.append(...plugins.map((plugin) => pluginCard(plugin, context)));
    view.replaceChildren(head(plugins.length), plugins.length ? grid : element('p', 'faint', say('plugins-page/none')));
  };
  load();
  return {
    update() {},
    event(name) {
      const typing = view.contains(document.activeElement) && document.activeElement.matches('input, textarea, select');
      if (name === 'cards' && !typing && !panel) load();
    },
    dispose() {
      disposed = true;
      panel?.close();
    },
  };
}

/** 插件声明的整页 `id`：上面一条返回详情、整页的标题，下面照界面块画（经插件写的方法取）。 */
function widgetPage(plugin, id, request) {
  const page = (plugin.ui?.pages || []).find((one) => one.id === id);
  const bar = element('div', 'panel-bar');
  const backTo = element('a', 'btn ghost');
  backTo.href = `#plugins/${encodeURIComponent(plugin.package)}`;
  backTo.append(icon('i-left'), document.createTextNode(named(plugin.name) || plugin.package));
  bar.append(backTo);
  if (!page) return [bar, element('p', 'faint', say('plugins-page/gone'))];
  const box = element('div', 'panel widget-page');
  box.append(element('h2', null, named(page.title)));
  const body = element('div');
  box.append(body);
  mountWidget(body, plugin.package, { place: 'page', page: id }, {
    request,
    openPage: (other) => {
      location.hash = `plugins/${encodeURIComponent(plugin.package)}/page/${encodeURIComponent(other)}`;
    },
  });
  return [bar, box];
}

/** 头上：标题、装了几个。 */
function head(count) {
  const shown = element('div', 'view-head');
  shown.append(element('h1', null, say('nav/plugins')));
  if (count !== null) shown.append(element('span', 'faint', say('plugins-page/count', { count })));
  return shown;
}

/** 回到列表。 */
export function back() {
  const button = element('a', 'btn ghost plug-back');
  button.href = '#plugins';
  button.append(icon('i-left'), document.createTextNode(say('nav/plugins')));
  return button;
}
