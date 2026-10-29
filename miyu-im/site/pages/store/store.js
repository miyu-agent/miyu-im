// 商城（施工 I-11，设计 01 第一节第 6 条第三页）：软件源里 im 的插件一张一张排开，能搜、能装（宿主的 store.list、store.install）。
// 2026-10-20 项目主人定：只列 im 的插件；装好留在商城（按钮变「已安装」，开、批能力照旧在插件页）；非官方的只标「第三方」。
//
// - 名字、说明核心已经照这个连接的语言挑好（package.search），照原样画。
// - 被下架的（blocked）不给装，标出来。
// - 软件源这一次没连上、照上一次读到的列的（stale），头上写一句。
import { element, icon, say } from '../../shell/i18n.js';

/** 在 `view` 里起商城：`request` 调宿主的方法。 */
export function mountStore(view, { request }) {
  let disposed = false;
  let asked = 0;
  const search = element('input', 'store-search');
  search.type = 'search';
  search.placeholder = say('store-page/search');
  const body = element('div', 'store-body');
  const top = element('div', 'page-head');
  top.append(element('h1', null, say('store-page/title')), search);
  view.replaceChildren(top, body);
  // 装着的、正在装的：重画时照旧。
  const installing = new Set();
  const load = async () => {
    const mine = ++asked;
    const query = search.value.trim();
    body.replaceChildren(element('p', 'faint', say('store-page/loading')));
    let found;
    try {
      found = await request('store.list', query ? { query } : {});
    } catch {
      if (!disposed && mine === asked) body.replaceChildren(element('p', 'faint', say('store-page/failed')));
      return;
    }
    if (disposed || mine !== asked) return;
    const packages = found.packages || [];
    const parts = [];
    if (found.stale) parts.push(element('p', 'faint store-stale', say('store-page/stale')));
    if (packages.length === 0) {
      parts.push(element('p', 'faint', say(query ? 'store-page/no-match' : 'store-page/none')));
    } else {
      const grid = element('div', 'plug-grid');
      grid.append(...packages.map((item) => storeCard(item, request, installing)));
      parts.push(grid);
    }
    body.replaceChildren(...parts);
  };
  let typing = 0;
  search.addEventListener('input', () => {
    clearTimeout(typing);
    typing = setTimeout(load, 300);
  });
  load();
  return {
    update() {},
    event() {},
    dispose() {
      disposed = true;
      clearTimeout(typing);
    },
  };
}

/** 商城里一个插件的卡片：名字、编号和版本、一句说明、官方还是第三方、装没装。 */
function storeCard(item, request, installing) {
  const card = element('div', 'plug-card store-card');
  const top = element('div', 'top');
  const box = element('span', 'ico');
  box.append(icon('i-puzzle'));
  const grow = element('div', 'grow');
  grow.append(element('b', null, item.name || item.package), element('div', 'faint', [item.package, item.version].filter(Boolean).join(' · ')));
  top.append(box, grow, action(item, request, installing));
  card.append(top);
  const summary = item.summary;
  if (summary) card.append(element('p', null, summary));
  const chips = element('div', 'chips');
  chips.append(element('span', item.official ? 'chip' : 'chip accent', say(item.official ? 'store-page/official' : 'store-page/third-party')));
  if (item.blocked) {
    const blocked = element('span', 'chip bad', say('store-page/blocked'));
    blocked.title = item.blocked;
    chips.append(blocked);
  }
  card.append(chips);
  return card;
}

/** 卡片右边：装着的写「已安装」，被下架的不给装，别的一个「安装」按钮（装的时候写「正在安装」，没装成的在按钮下面说一句）。 */
function action(item, request, installing) {
  const holder = element('div', 'store-action');
  const draw = (state, failed) => {
    if (state === 'installed') {
      holder.replaceChildren(element('span', 'chip good', say('store-page/installed')));
      return;
    }
    const button = element('button', 'btn primary', say(state === 'installing' ? 'store-page/installing' : 'store-page/install'));
    button.disabled = state === 'installing' || Boolean(item.blocked);
    button.addEventListener('click', async (event) => {
      event.stopPropagation();
      installing.add(item.package);
      draw('installing');
      try {
        await request('store.install', { package: item.package });
        item.installed = item.version || '';
        draw('installed');
      } catch {
        draw('idle', true);
      } finally {
        installing.delete(item.package);
      }
    });
    holder.replaceChildren(button);
    if (failed) holder.append(element('div', 'store-failed', say('store-page/install-failed')));
  };
  if (item.installed !== null && item.installed !== undefined) draw('installed');
  else draw(installing.has(item.package) ? 'installing' : 'idle');
  return holder;
}
