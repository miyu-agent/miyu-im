// 一块插件内容的外壳（施工 I-8 下）：经宿主的 plugins.widget 取（宿主调插件登记的方法，照核心的界面块查过、照语言挑好字），画进
// `box`（widgets.js）；点了动作、存了表单以后重取。等不到的、写错的那个插件的那一块写一句，别的照常。
import { element, say } from '../../shell/i18n.js';
import { drawWidget } from './widgets.js';

/**
 * 在 `box` 里画插件 `package` 的一块：`where` 是 { place: 'home' | 'panel' | 'page', venue?: {type, id}, page? }。`options` 是
 * { request, openPage(id) }。交回 { refresh() }。
 */
export function mountWidget(box, packageId, where, { request, openPage }) {
  let content = null;
  const context = {
    state: {},
    call: (method, params) => request('plugins.call', { package: packageId, method, ...(params === undefined ? {} : { params }) }),
    open: (id) => openPage(id),
    refresh: () => load(),
    redraw: () => draw(),
  };
  const draw = () => {
    if (content === null) return;
    const parts = drawWidget(content, context);
    box.replaceChildren(...(parts.length > 0 ? parts : [element('p', 'faint', say('plugins-page/widget-empty'))]));
  };
  const load = async () => {
    try {
      const answer = await request('plugins.widget', { package: packageId, ...where, language: document.documentElement.lang || 'en' });
      content = answer.content ?? [];
    } catch (reason) {
      box.replaceChildren(element('p', 'faint', say(reason === 'bad_widget_content' ? 'plugins-page/widget-bad' : 'plugins-page/widget-failed')));
      return;
    }
    draw();
  };
  if (!box.firstChild) box.append(element('p', 'faint', say('plugins-page/loading')));
  load();
  return { refresh: load };
}
