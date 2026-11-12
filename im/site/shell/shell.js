// 页面的外壳（设计 01 第八节第 1 条，施工 I-6 上）：左边侧边栏（头上名字和在线状态、几页的入口、底下切深浅、打开主网页、源码），右边是
// 这一页。施工 I-110（项目主人 11-02）：去掉「登出」（只给一个人用）；源码换成 GitHub 的标志；装了网页软件的多一个「主网页」，带一张
// 交接票打开（宿主的 web.open），那边直接是登录的；深浅只在浅色、深色之间切，「跟随系统」在设置页「外观」里选。
//
// - 收起、展开照网页软件的左栏（shell.css）；收起着、指针靠到最左边，左栏临时浮出来，离开了滑回去。收没收起记进 localStorage。
// - 哪一页照网址的 #（#home 这样），前进后退照浏览器的。还没做的几页在侧边栏上灰着，写「即将」。
// - 连接的样子由外壳问（connections），宿主推了变了再问；头上的状态、这一页都照它画。通道断了头上写连不上。宿主推的别的（cards：
//   卡片变了，施工 I-7 上）交给这一页。
import { element, icon, say } from './i18n.js';
import { choose, shown } from './theme.js';
import { toast } from './dialog.js';
import { lucide } from './lucide.js';
import { goTo, hashParts } from './connection.js';
import { mountHome } from '../pages/home/home.js';
import { mountVenues } from '../pages/venues/venues.js';
import { mountRecords } from '../pages/records/records.js';
import { mountSettings } from '../pages/settings/settings.js';
import { mountPlugins } from '../pages/plugins/plugins.js';
import { mountStore } from '../pages/store/store.js';
import { mountStats } from '../pages/stats/stats.js';
import { mountLimits } from '../pages/limits/limits.js';
import { mountPersonas } from '../pages/personas/personas.js';
import { mountPresets } from '../pages/presets/presets.js';
import { mountModels } from '../pages/models/models.js';
import { mountBackstage } from '../pages/backstage/backstage.js';

/** 侧边栏的几页：编号、名字、做好了的画法（没有的灰着）、分在哪一组（施工 I-150，2026-11-05 项目主人：十二项太长，照用途分组加小
 *  标题；没写的是头一组，不带标题；`end` 只隔开一点、不带标题）。施工 I-153（项目主人 11-05）：「扩展」改叫「软件」，下面
 *  照内核的叫法写已安装、软件后台、下载软件，还是只管 im 的插件；网址里的编号（plugins、backstage、store）不变。 */
const PAGES = [
  { id: 'home', name: () => say('nav/home'), mount: mountHome },
  { id: 'venues', name: () => say('nav/venues'), mount: mountVenues },
  { id: 'records', name: () => say('nav/records'), mount: mountRecords },
  { id: 'stats', name: () => say('nav/stats'), mount: mountStats },
  { id: 'limits', name: () => say('nav/limits'), mount: mountLimits },
  { id: 'personas', name: () => say('nav/personas'), mount: mountPersonas, group: 'ai' },
  { id: 'presets', name: () => say('nav/presets'), mount: mountPresets, group: 'ai' },
  { id: 'models', name: () => say('nav/models'), mount: mountModels, group: 'ai' },
  { id: 'plugins', name: () => say('nav/plugins'), mount: mountPlugins, group: 'extend' },
  { id: 'backstage', name: () => say('nav/backstage'), mount: mountBackstage, group: 'extend' },
  { id: 'store', name: () => say('nav/store'), mount: mountStore, group: 'extend' },
  { id: 'settings', name: () => say('nav/settings'), mount: mountSettings, group: 'end' },
];

const COLLAPSED = 'miyu-im-collapsed';
const SCHEME_ICONS = { light: 'i-sun', dark: 'i-moon' };

/** 这个程序的源码。 */
const SOURCE = 'https://github.com/miyu-agent/miyu-im';

/** 在 `root` 里起外壳：`request` 调宿主的方法，`handed` 是网页软件交来了主题（不给手动切）。交回 refresh()（变了再问）、
 *  event(name)（宿主推的别的）、offline()（通道断了）、dispose()。 */
export function mountShell(root, { request, handed }) {
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
  let group;
  for (const page of PAGES) {
    if (page.group !== group) {
      group = page.group;
      if (group === 'end') side.append(element('div', 'nav-gap'));
      else if (group) side.append(element('div', 'nav-group', say(`nav/group-${group}`)));
    }
    const nav = element('button', 'nav', page.name());
    if (!page.mount) {
      nav.disabled = true;
      nav.append(element('span', 'soon', say('nav/soon')));
    } else {
      nav.addEventListener('click', () => {
        // 选中的连接跟着网址走（施工 I-85 中）。
        goTo(page.id);
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
  // 主网页（施工 I-110）：装了网页软件才出现。先开一个空的新标签页（等宿主要票再开会被浏览器当成弹窗挡掉），要到了地址再换过去。
  const web = element('button', 'btn ghost');
  web.append(lucide('globe', icon('i-link')), document.createTextNode(say('side/web')));
  web.hidden = true;
  web.addEventListener('click', async () => {
    const tab = window.open('', '_blank');
    try {
      const { url } = await request('web.open');
      if (tab) {
        tab.opener = null;
        tab.location.href = url;
      } else location.href = url;
    } catch {
      tab?.close();
      toast(say('failed/web'));
    }
  });
  request('web.status').then(
    (answer) => {
      web.hidden = !answer.installed;
    },
    () => {},
  );
  // 源码（2026-10-26 换 AGPL-3.0-or-later）：经网络用到这个页面的人也拿得到源码。
  const source = element('a', 'icon-button');
  source.href = SOURCE;
  source.target = '_blank';
  source.rel = 'noopener';
  source.title = say('source');
  source.append(icon('i-github'));
  foot.append(element('span', 'grow'), web, source);
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

  // 深浅：照现在显示的换成另一个（施工 I-110）；图标画现在的样子，悬停说点了换成什么。
  const showScheme = () => {
    const now = shown();
    scheme.replaceChildren(icon(SCHEME_ICONS[now]));
    scheme.title = say(now === 'dark' ? 'theme/to-light' : 'theme/to-dark');
  };
  scheme.addEventListener('click', () => {
    choose(shown() === 'dark' ? 'light' : 'dark');
    showScheme();
  });
  window.addEventListener('miyu-im-scheme', showScheme);
  showScheme();

  // 哪一页。
  let connections = null;
  let current = null;
  const go = () => {
    // #plugins/<编号> 这样的：头一段是哪一页，后面交给那一页（施工 I-9 上：插件详情）。
    const [id, ...rest] = hashParts().route.split('/');
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
