// 插件详情（施工 I-9 上，照样板 d-im.html 的插件详情）：头上图标、名字、编号和版本、主页仓库问题反馈三个链接、总开关；左边三页
// 「介绍」（README，readme.js）、「设置」（插件自己的配置项，施工 I-9 中，config-form.js）、「用在哪」（每个会话一个开关，同会话浮窗
// 「管理插件」的算法），右边「关于」。
//
// - README 宿主经桥自己的连接读（plugins.readme），读过的记着，重画时不再读；没写的照一句说明。
// - 开着、没在跑的（缺批准、失败停下），头下面说一句为什么。
import { element, icon, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { face } from '../venues/face.js';
import { entries } from '../venues/model.js';
import { idLine, titleOf } from '../venues/names.js';
import { fallbacksOf, isOn, named, pluginSwitch } from '../venues/plugins.js';
import { back } from './plugins.js';
import { pluginIcon, subtitle, usable } from './card.js';
import { drawSettings } from './config-form.js';
import { readme } from './readme.js';
import { masterSwitch } from './switch.js';

/** 读过的 README：编号 → 字（读不成的是空的）。 */
const READMES = new Map();

/** 记着开着哪一页：重画时照旧。 */
let tab = 'readme';

/** 插件 `plugin` 的详情：`context` 是 { request, platform, cards（cards.get 的回答）, reload }。 */
export function drawDetail(plugin, context) {
  // 「会话默认值」那张卡不是一个会话：不列在「用在哪」里。
  const all = entries(context.cards).filter((one) => one.kind !== 'defaults');
  const parts = [back(), detailHead(plugin, context)];
  if (plugin.enabled && plugin.state && plugin.state !== 'running') {
    const why = plugin.reason ? say(`plugins-page/stopped-${plugin.reason}`) : say('plugins-page/not-running');
    parts.push(element('p', 'plug-notice', why));
  }
  const two = element('div', 'two');
  two.append(tabs(plugin, all, context), about(plugin, all, context.platform, context));
  parts.push(two);
  return parts;
}

/** 头上一块。 */
function detailHead(plugin, context) {
  const head = element('div', 'detail-head');
  const grow = element('div', 'grow');
  grow.append(element('h2', null, named(plugin.name) || plugin.package), element('div', 'soft', subtitle(plugin)));
  const links = element('div', 'links');
  for (const kind of ['homepage', 'repository', 'issues']) {
    const href = plugin.links?.[kind];
    if (!href || !/^https?:\/\//i.test(href)) continue;
    const anchor = element('a', null, say(`plugins-page/link-${kind}`));
    anchor.href = href;
    anchor.target = '_blank';
    anchor.rel = 'noopener noreferrer';
    anchor.append(icon('i-up-right'));
    links.append(anchor);
  }
  if (links.childElementCount > 0) grow.append(links);
  head.append(pluginIcon(plugin), grow, masterSwitch(plugin, context));
  // 声明的整页（施工 I-8 下，2026-10-20 项目主人定：详情头上一个整页一个按钮）。
  for (const page of plugin.enabled ? plugin.ui?.pages || [] : []) {
    const open = element('a', 'btn open-panel');
    open.href = `#plugins/${encodeURIComponent(plugin.package)}/page/${encodeURIComponent(page.id)}`;
    open.textContent = named(page.title) || page.id;
    head.append(open);
  }
  // 有后台页、开着的：打开面板（施工 I-9 下）。
  if (plugin.page && plugin.enabled) {
    const open = element('a', 'btn primary open-panel');
    open.href = `#plugins/${encodeURIComponent(plugin.package)}/panel`;
    open.append(document.createTextNode(say('plugins-page/open-panel')), icon('i-up-right'));
    head.append(open);
  }
  return head;
}

/** 左边的两页。 */
function tabs(plugin, all, context) {
  const panel = element('div', 'panel');
  const seg = element('div', 'seg plug-tabs');
  const body = element('div');
  const pages = {
    readme: () => intro(plugin, context.request),
    settings: () => [drawSettings(plugin.package, context.request)],
    where: () => where(plugin, all, context),
  };
  const show = (key) => {
    tab = key;
    for (const button of seg.children) button.classList.toggle('on', button.dataset.tab === key);
    body.replaceChildren(...pages[key]());
  };
  for (const key of Object.keys(pages)) {
    const button = element('button', null, say(`plugins-page/tab-${key}`));
    button.dataset.tab = key;
    button.addEventListener('click', () => show(key));
    seg.append(button);
  }
  panel.append(seg, body);
  show(pages[tab] ? tab : 'readme');
  return panel;
}

/** 「介绍」：README；没写的照一句说明。 */
function intro(plugin, request) {
  const box = element('div', 'readme');
  const fill = (text) => box.replaceChildren(...(text ? readme(text) : [element('p', null, named(plugin.summary) || say('plugins-page/no-readme'))]));
  if (!plugin.links?.readme) {
    fill('');
  } else if (READMES.has(plugin.package)) {
    fill(READMES.get(plugin.package));
  } else {
    box.append(element('p', 'faint', say('plugins-page/loading')));
    request('plugins.readme', { package: plugin.package }).then(
      (answer) => {
        READMES.set(plugin.package, answer.text || '');
        fill(answer.text || '');
      },
      () => fill(''),
    );
  }
  return [box];
}

/** 「用在哪」：每个会话一行，头像、名字和号、开关。 */
function where(plugin, all, { request, platform }) {
  const rows = [element('p', 'faint pane-hint', say('plugins-page/where-hint'))];
  if (plugin.unusable) rows.push(element('p', 'plugin-why', usable(plugin, '')));
  const fallbacks = fallbacksOf(all);
  for (const one of all) {
    const line = element('div', 'row');
    const label = element('div', 'label', titleOf(one));
    label.append(element('small', null, idLine(one, { platform, bots: new Map() })));
    const fallback = one.id ? fallbacks[one.type] || {} : {};
    line.append(face(one.type, one.id, one.name), label, pluginSwitch(one, plugin, fallback, request));
    rows.push(line);
  }
  return rows;
}

/** 右边「关于」。 */
function about(plugin, all, host, context) {
  const panel = element('div', 'panel plug-about');
  panel.append(element('h3', null, say('plugins-page/about')));
  const fallbacks = fallbacksOf(all);
  // 只数名单里的会话，不数「陌生群」「陌生私聊」两张兜底卡（2026-10-20 项目主人：会话页的张数也不数它们）。
  const listed = all.filter((one) => one.id);
  const on = listed.filter((one) => isOn(one, plugin, fallbacks[one.type] || {})).length;
  const points = (plugin.points || []).map((point) => say(`plugins-page/point-${point}`));
  const capabilities = (plugin.capabilities || []).map((one) => one.name || one.id);
  const rows = [
    ['version', plugin.version || say('plugins-page/unknown')],
    ['author', plugin.author || say('plugins-page/unknown')],
    ['from', say(`plugins-page/from-${plugin.from || 'local'}`)],
    ['points', points.join(say('plugins-page/separator')) || say('plugins-page/nothing')],
    ['platform', usable(plugin, host)],
    ['venues', `${on} / ${listed.length}`],
    ['capabilities', capabilities.join(say('plugins-page/separator')) || say('plugins-page/nothing')],
  ];
  for (const [key, value] of rows) {
    const row = element('div', 'meta-row');
    row.append(element('span', null, say(`plugins-page/about-${key}`)), element('b', key === 'platform' && plugin.unusable ? 'bad' : null, value));
    panel.append(row);
  }
  if (plugin.ui?.home) panel.append(homeSwitch(plugin, context));
  return panel;
}

/** 「放在首页」（施工 I-8 下，2026-10-20 项目主人定：只在插件详情里开关）。 */
function homeSwitch(plugin, context) {
  const row = element('div', 'meta-row home-switch');
  const button = element('button', plugin.on_home !== false ? 'switch on' : 'switch');
  button.append(element('i'));
  button.addEventListener('click', async () => {
    button.disabled = true;
    try {
      await context.request('plugins.home', { package: plugin.package, shown: plugin.on_home === false });
    } catch {
      toast(say('failed/save'));
    }
    context.reload();
  });
  row.append(element('span', null, say('plugins-page/on-home')), button);
  return row;
}
