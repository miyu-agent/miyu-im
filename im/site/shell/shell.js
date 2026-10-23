// 页面的外壳（设计 01 第八节第 1 条，施工 I-6 上）：左边侧边栏（头上名字和在线状态、几页的入口、底下切深浅和登出），右边是这一页。
//
// - 收起、展开照网页软件的左栏（shell.css）；收起着、指针靠到最左边，左栏临时浮出来，离开了滑回去。收没收起记进 localStorage。
// - 哪一页照网址的 #（#home 这样），前进后退照浏览器的。还没做的几页在侧边栏上灰着，写「即将」。
// - 连接的样子由外壳问（connections），宿主推了变了再问；头上的状态、这一页都照它画。通道断了头上写连不上。宿主推的别的（cards：
//   卡片变了，施工 I-7 上）交给这一页。
import { element, icon, say } from './i18n.js';
import { choose, chosen } from './theme.js';
import { mountHome } from '../pages/home/home.js';
import { mountVenues } from '../pages/venues/venues.js';
import { mountRecords } from '../pages/records/records.js';
import { mountSettings } from '../pages/settings/settings.js';
import { mountPlugins } from '../pages/plugins/plugins.js';
import { mountStore } from '../pages/store/store.js';
import { mountStats } from '../pages/stats/stats.js';

/** 侧边栏的几页：编号、名字、做好了的画法（没有的灰着）。 */
const PAGES = [
  { id: 'home', name: () => say('nav/home'), mount: mountHome },
  { id: 'venues', name: () => say('nav/venues'), mount: mountVenues },
  { id: 'records', name: () => say('nav/records'), mount: mountRecords },
  { id: 'plugins', name: () => say('nav/plugins'), mount: mountPlugins },
  { id: 'store', name: () => say('nav/store'), mount: mountStore },
  { id: 'stats', name: () => say('nav/stats'), mount: mountStats },
  { id: 'settings', name: () => say('nav/settings'), mount: mountSettings },
];

const COLLAPSED = 'miyu-im-collapsed';
const SCHEMES = ['system', 'light', 'dark'];
const SCHEME_ICONS = { system: 'i-monitor', light: 'i-sun', dark: 'i-moon' };
const SCHEME_NAMES = { system: () => say('theme/system'), light: () => say('theme/light'), dark: () => say('theme/dark') };

/** 在 `root` 里起外壳：`request` 调宿主的方法，`handed` 是网页软件交来了主题（不给手动切），`onSignOut` 登出。交回
 *  refresh()（变了再问）、event(name)（宿主推的别的）、offline()（通道断了）、dispose()。 */
export function mountShell(root, { request, handed, onSignOut }) {
  document.title = say('title');
  const shell = element('div', 'shell');
  const toggle = element('button', 'icon-button side-toggle');
  toggle.append(icon('i-panel'));
  const peekZone = element('div', 'peek-zone');
  const side = element('aside', 'side');
  const brandDot = element('span', 'dot');
  const brandStatus = element('span');
  const brand = element('header', 'brand-row');
  const named = element('div', 'brand');
  const status = element('span');
  status.append(brandDot, brandStatus);
  named.append(element('strong', null, say('title')), status);
  brand.append(element('span', 'toggle-slot'), named);
  side.append(brand);
  const navs = new Map();
  for (const page of PAGES) {
    const nav = element('button', 'nav', page.name());
    if (!page.mount) {
      nav.disabled = true;
      nav.append(element('span', 'soon', say('nav/soon')));
    } else {
      nav.addEventListener('click', () => {
        location.hash = page.id;
        // 窄屏上左栏盖着内容：点了就收起。
        if (narrow.matches) collapse(true, false);
      });
    }
    navs.set(page.id, nav);
    side.append(nav);
  }
  const foot = element('div', 'side-foot');
  const scheme = element('button', 'icon-button');
  if (!handed) foot.append(scheme);
  const out = element('button', 'btn ghost', say('sign-out'));
  out.addEventListener('click', onSignOut);
  foot.append(element('span', 'grow'), out);
  side.append(foot);
  const main = element('main', 'main');
  shell.append(toggle, peekZone, side, main);
  root.replaceChildren(shell);
  root.hidden = false;

  // 收起、展开、浮出来。
  let peekTimer = 0;
  // 收起（`yes`）、展开；`remember` 的记下来（人点的才记，窄屏自己收起的不记）。
  const collapse = (yes, remember) => {
    clearTimeout(peekTimer);
    shell.classList.toggle('collapsed', yes);
    shell.classList.remove('peek');
    toggle.title = yes ? say('side/expand') : say('side/collapse');
    if (!remember) return;
    try {
      localStorage.setItem(COLLAPSED, yes ? '1' : '');
    } catch {
      // 存不了：这一回照样用。
    }
  };
  const peek = (inside) => {
    clearTimeout(peekTimer);
    if (!shell.classList.contains('collapsed')) return;
    peekTimer = setTimeout(() => shell.classList.toggle('peek', inside), inside ? 120 : 260);
  };
  toggle.addEventListener('click', () => collapse(!shell.classList.contains('collapsed'), !narrow.matches));
  for (const zone of [peekZone, toggle, side]) {
    zone.addEventListener('mouseenter', () => peek(true));
    zone.addEventListener('mouseleave', () => peek(false));
  }
  const narrow = window.matchMedia('(max-width: 720px)');
  let wasCollapsed = narrow.matches;
  try {
    wasCollapsed ||= localStorage.getItem(COLLAPSED) === '1';
  } catch {
    // 读不了：宽屏展开着。
  }
  collapse(wasCollapsed, false);

  // 深浅：跟随系统、浅色、深色轮着切。
  const showScheme = () => {
    const now = chosen();
    scheme.replaceChildren(icon(SCHEME_ICONS[now]));
    scheme.title = SCHEME_NAMES[now]();
  };
  scheme.addEventListener('click', () => {
    choose(SCHEMES[(SCHEMES.indexOf(chosen()) + 1) % SCHEMES.length]);
    showScheme();
  });
  showScheme();

  // 哪一页。
  let connections = null;
  let current = null;
  const go = () => {
    // #plugins/<编号> 这样的：头一段是哪一页，后面交给那一页（施工 I-9 上：插件详情）。
    const [id, ...rest] = location.hash.slice(1).split('/');
    const wanted = PAGES.find((page) => page.mount && page.id === id) || PAGES[0];
    const path = rest.length > 0 ? decodeURIComponent(rest.join('/')) : '';
    for (const [id, nav] of navs) nav.classList.toggle('on', id === wanted.id);
    current?.dispose?.();
    const view = element('section', 'view');
    main.replaceChildren(view);
    main.scrollTop = 0;
    current = wanted.mount(view, { request, path });
    if (connections) current.update(connections);
  };
  window.addEventListener('hashchange', go);
  go();

  const showStatus = (online) => {
    if (!online) {
      brandDot.className = 'dot off';
      brandStatus.textContent = say('status/offline');
    } else if (!connections || connections.length === 0) {
      brandDot.className = 'dot';
      brandStatus.textContent = say('status/none');
    } else {
      // 断开在重连的（施工 I-6 中）：一部分是琥珀色，全断了是红的。
      const gone = connections.filter((one) => !one.connected).length;
      brandDot.className = gone === 0 ? 'dot on' : gone === connections.length ? 'dot off' : 'dot warn';
      brandStatus.textContent = gone === 0
        ? say('status/online', { count: connections.length })
        : say('status/reconnecting', { count: connections.length, gone });
    }
  };

  return {
    async refresh() {
      try {
        connections = (await request('connections')).connections;
      } catch {
        return;
      }
      showStatus(true);
      current?.update(connections);
    },
    event(name) {
      current?.event?.(name);
    },
    offline() {
      showStatus(false);
    },
    dispose() {
      window.removeEventListener('hashchange', go);
      current?.dispose?.();
    },
  };
}
