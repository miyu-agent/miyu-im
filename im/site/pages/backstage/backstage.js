// 插件后台（施工 I-102，项目主人 11-02 定：侧栏一页，方便的插件后台入口）：开着的插件的后台页、整页一个一张（entries.js），点了在这一页
// 里打开（#backstage/<编号>/panel、#backstage/<编号>/page/<整页的编号>），左上角回这一页。面板、整页照插件页的画法（panel.js、
// plugins.js 的 widgetPage），只是返回的地方不同；侧栏停在「插件后台」。
import { element, icon, say } from '../../shell/i18n.js';
import { pluginIcon } from '../plugins/card.js';
import { drawPanel } from '../plugins/panel.js';
import { widgetPage } from '../plugins/plugins.js';
import { named } from '../venues/plugins.js';
import { backstages } from './entries.js';

/** 在 `view` 里起插件后台：`request` 调宿主的方法，`path` 是网址里 #backstage/ 后面的那一段（没有的是列表）。 */
export function mountBackstage(view, { request, path }) {
  let disposed = false;
  const [wanted, sub, pageId] = path.split('/');
  // 开着的面板：只画一次，页面换走时收掉。
  let panel = null;
  let connections = [];
  const back = { href: '#backstage', label: say('nav/backstage') };
  const load = async () => {
    let listed;
    let status;
    try {
      [listed, status] = await Promise.all([request('plugins.list'), request('status')]);
    } catch {
      if (!disposed) view.replaceChildren(head(), element('p', 'faint', say('plugins-page/failed')));
      return;
    }
    if (disposed || panel) return;
    const plugins = listed.plugins || [];
    if (wanted) {
      const plugin = plugins.find((one) => one.package === wanted);
      if (!plugin || backstages(plugin).length === 0) {
        view.replaceChildren(backLink(back), element('p', 'faint', say('plugins-page/gone')));
      } else if (sub === 'page') {
        panel = { close() {} };
        view.replaceChildren(...widgetPage(plugin, pageId, request, { back, base: 'backstage' }));
      } else {
        const context = { request, platform: status.platform || '', connections: () => connections, back };
        panel = drawPanel(plugin, context);
        view.classList.add('panel-view');
        view.replaceChildren(...panel.parts);
      }
      return;
    }
    const tiles = plugins.flatMap((plugin) => backstages(plugin).map((entry) => tile(plugin, entry)));
    const grid = element('div', 'plug-grid');
    grid.append(...tiles);
    view.replaceChildren(head(), tiles.length ? grid : element('p', 'page-none', say('backstage/none')));
  };
  load();
  return {
    update(latest) {
      connections = latest || [];
    },
    dispose() {
      disposed = true;
      panel?.close();
    },
  };
}

/** 一个后台一张：图标、插件的名字（整页的再写页名）、一句说明。 */
function tile(plugin, entry) {
  const card = element('a', 'plug-card backstage-card');
  card.href = entry.href;
  const top = element('div', 'top');
  const grow = element('div', 'grow');
  const name = named(plugin.name) || plugin.package;
  // 面板名已经带着软件名的（「群管理」的「群管理面板」）只写面板名（施工 I-170：原来写成「群管理 · 群管理面板」）。
  const title = entry.page ? named(entry.page.title) || entry.page.id : name;
  grow.append(element('b', null, title.startsWith(name) ? title : `${name} · ${title}`));
  top.append(pluginIcon(plugin), grow, icon('i-right'));
  card.append(top);
  const summary = named(plugin.summary);
  if (summary) card.append(element('p', null, summary));
  return card;
}

/** 头上：标题。 */
function head() {
  const shown = element('div', 'view-head');
  shown.append(element('h1', null, say('nav/backstage')));
  return shown;
}

/** 回到列表。 */
function backLink(back) {
  const button = element('a', 'btn ghost plug-back');
  button.href = back.href;
  button.append(icon('i-left'), document.createTextNode(back.label));
  return button;
}
