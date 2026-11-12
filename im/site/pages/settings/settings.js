// 设置页（施工 I-7 下一，设计 01 第八节第 6 条）：只放 im 自己的、各平台通用的设置。几段：本人的平台 ID（核心的 `external.bindings`，
// 施工 I-68）、通用（回执自动撤回的命令）、外观（施工 I-110：跟随系统、浅色、深色）、访问（施工 I-167：向局域网开放、端口、局域网
// 网址和二维码）。每一段是一张共用的卡片（shell/sect.js，设计 11 G1）。违规关键词
// 施工 I-113 起挪到群管理插件的设置里（plugins/moderation.js）。照样板左边一列切段、右边一次一段
// （施工 I-7 下补）；切到哪一段重画时照旧。会话默认值（施工 I-7 下二）2026-10-20 挪到会话页墙上第一张卡（venues/）。
//
// - 本人的平台 ID 列每个平台的身份（施工 I-85 补，设计 07 第七节 im-己）：平台不止一个时每行写平台，加的那一行先选平台（默认是
//   首页选中的连接的平台）。平台照外壳问到的连接。
// - 读写都经宿主的 settings.get、settings.set（宿主在这份登录的人的连接上改核心的配置）。改了当场存，存不成的提示一句、重读一遍；
//   回执撤回的命令是标签（shell/tags.js），加了当场画上，存成了不重读（重读会把正在填的那一格换掉），存不成的才重读。
import { element, icon, say, spaced } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { tagInput } from '../../shell/tags.js';
import { platform, venueKinds } from '../home/platforms.js';
import { chosenOf } from '../../shell/connection.js';
import { choose, chosen, handed } from '../../shell/theme.js';
import { openBind } from './bind.js';
import { lanCard } from './lan-card.js';
import { sect } from '../../shell/sect.js';

/** 在 `view` 里起设置页：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountSettings(view, { request }) {
  // 外壳问到的连接；加身份时选着的平台（没选过的照首页选中的连接）。
  let connections = [];
  let adding = '';
  // 正开着的一段。
  let page = 'bindings';
  // 问到的：连接变了照它重画，不再问。
  let got = null;
  const draw = () => {
    if (!got) return;
    const platforms = [...new Set(connections.map((one) => one.platform))];
    if (!platforms.includes(adding)) adding = chosenOf(connections)?.platform || platforms[0] || '';
    const pages = [
      ['bindings', bindings(got, { platforms, adding }, save)],
      ['general', general(got, save)],
      ['appearance', appearance()],
      ['site', site(got, save)],
    ];
    view.replaceChildren(head(), tabbed(pages, page, (key) => (page = key)));
  };
  const load = async () => {
    try {
      got = await request('settings.get');
    } catch {
      view.replaceChildren(head(), element('p', 'faint', say('settings-page/failed')));
      return;
    }
    draw();
  };
  // `quiet` 的存成了不重读（标签当场画上了）。
  const save = async (key, value, quiet) => {
    let saved = false;
    try {
      saved = (await request('settings.set', { key, value })).saved;
      if (!saved) toast(say('failed/save'));
    } catch {
      toast(say('failed/save'));
    }
    if (!saved || !quiet) load();
    return saved;
  };
  load();
  return {
    update(now) {
      connections = now;
      draw();
    },
    event() {},
    dispose() {},
  };
}

/** 左边一列切段（`pages` 是 `[编号, 那一段]`），开着 `open` 那一段；切了告诉 `chose`。 */
function tabbed(pages, open, chose) {
  const box = element('div', 'set');
  const nav = element('div', 'set-nav');
  const buttons = pages.map(([key, panel]) => {
    const button = element('button', key === open ? 'd-tab on' : 'd-tab', say(`settings-page/${key}`));
    panel.hidden = key !== open;
    button.addEventListener('click', () => {
      for (const [other, shown] of pages) shown.hidden = other !== key;
      for (const each of buttons) each.classList.toggle('on', each === button);
      chose(key);
    });
    return button;
  });
  nav.append(...buttons);
  box.append(nav, element('div', 'set-body'));
  box.lastChild.append(...pages.map(([, panel]) => panel));
  return box;
}

function head() {
  const shown = element('div', 'view-head');
  shown.append(element('h1', null, say('nav/settings')));
  return shown;
}

/** 一段：一张卡片（shell/sect.js），组名、小字、下面的几行。 */
function section(title, hint, rows) {
  const card = sect(say(title), hint ? say(hint) : null);
  card.body.append(...rows);
  return card.box;
}

/** 一行：名字、小字、控件。 */
function row(label, hint, control) {
  const line = element('div', 'row');
  const text = element('div', 'label', label);
  if (hint) text.append(element('small', null, hint));
  line.append(text, control);
  return line;
}

/** 本人的平台 ID（核心的 `external.bindings`，施工 I-68）：一个身份一行（号、去掉），最后一行加一个。只有一个主人，不写账号。平台不止一个时
 *  （施工 I-85 补）每行写平台：`platforms` 是挂着的平台，`adding` 是新绑定的浮窗先选着的平台（施工 I-111 起浮窗里选）。 */
function bindings(got, { platforms, adding }, save) {
  const several = platforms.length > 1;
  const rows = [];
  for (const identity of got.bindings || []) {
    const at = identity.indexOf(':');
    const on = identity.slice(0, at);
    const number = identity.slice(at + 1);
    const word = venueKinds(on).private?.id() || say('venues/user-id');
    const remove = element('button', 'icon-button');
    remove.title = say('settings-page/unbind');
    remove.append(icon('i-x'));
    remove.addEventListener('click', () => save(`binding:${identity}`, null));
    const label = several ? `${platform(on).name()} · ${word} ${number}` : `${word} ${number}`;
    rows.push(row(spaced(label), '', remove));
  }
  // 新绑定（施工 I-111，项目主人 11-02）：点了开浮窗，第一步选平台、第二步填 ID（bind.js）。
  const add = element('button', 'btn', say('settings-page/bind'));
  add.disabled = platforms.length === 0;
  add.addEventListener('click', () => openBind(platforms, adding, (on, id) => save(`binding:${on}:${id}`, true)));
  rows.push(row(say('settings-page/bind-new'), '', add));
  return section('settings-page/bindings', 'settings-page/bindings-hint', rows);
}

/** 通用：去掉末尾句号、模拟真人一条分多条发送（施工 I-26）、回执自动撤回的命令。 */
function general(got, save) {
  const toggle = (key) => {
    const button = element('button', got[key] ? 'switch on' : 'switch');
    button.append(element('i'));
    button.addEventListener('click', () => save(key, !got[key]));
    return button;
  };
  const tags = tagInput({
    items: got.recall_receipts || [],
    normalize: (typed) => typed.replace(/^\//, '').trim(),
    save: (list) => save('recall_receipts', list, true),
  });
  return section('settings-page/general', null, [
    row(say('settings-page/recall'), say('settings-page/recall-hint'), tags),
  ]);
}

/** 外观（施工 I-110，项目主人 11-02）：深浅三档，跟随系统、浅色、深色，记在这台浏览器上（shell/theme.js）。从网页软件打开的照它的
 *  颜色，不给切。 */
function appearance() {
  const seg = element('div', 'seg');
  const draw = () => {
    seg.replaceChildren(
      ...['system', 'light', 'dark'].map((one) => {
        const button = element('button', one === chosen() ? 'on' : null, say(`theme/${one}`));
        button.disabled = handed();
        button.addEventListener('click', () => {
          choose(one);
          draw();
        });
        return button;
      }),
    );
  };
  draw();
  const hint = handed() ? say('settings-page/scheme-handed') : null;
  return section('settings-page/appearance', null, [row(say('settings-page/scheme'), hint, seg)]);
}

/** 访问（施工 I-167）：向局域网开放（`im.web.listen` 写 `0.0.0.0`，关了写回 `127.0.0.1`）、端口；开着的再给一张局域网网址的卡
 *  （lan-card.js），问不到地址的写一句。局域网来的用核心的访问密码登录。施工 I-193（照网页软件 W-1135）：每块网卡的地址都列，卡照
 *  第一个画、网址后面暗字写网卡名，别的写在卡下面「其他网卡：网址（网卡名）、…」；im 跑在 macOS 上的多一句去哪放行防火墙。 */
function site(got, save) {
  const open = got.listen === '0.0.0.0';
  const lan = element('button', open ? 'switch on' : 'switch');
  lan.append(element('i'));
  lan.addEventListener('click', () => save('listen', open ? '127.0.0.1' : '0.0.0.0'));
  const port = element('input', 'input num');
  port.inputMode = 'numeric';
  port.value = got.port ?? '';
  port.addEventListener('change', () => {
    const number = Number(port.value.trim());
    if (Number.isInteger(number) && number > 0 && number < 65536) save('port', number);
    else toast(say('failed/invalid'));
  });
  const rows = [row(say('settings-page/lan'), say('settings-page/lan-hint'), lan), row(say('settings-page/port'), say('settings-page/port-hint'), port)];
  if (open) rows.push(...lanRows(got));
  return section('settings-page/site', 'settings-page/site-hint', rows);
}

/** 开着局域网时卡片下面的几行（施工 I-193）：`got.lans` 是 `[{address, card}]`，`got.lan` 是第一个的地址。 */
function lanRows(got) {
  if (!got.lan) return [element('p', 'faint', say('settings-page/lan-unknown'))];
  const all = Array.isArray(got.lans) ? got.lans.filter((one) => typeof one?.address === 'string') : [];
  const url = (address) => `http://${address}/`;
  const mine = all.find((one) => one.address === got.lan)?.card ?? null;
  const others = all
    .filter((one) => one.address !== got.lan)
    .map((one) => (one.card ? `${url(one.address)}（${one.card}）` : url(one.address)));
  const rows = [lanCard(url(got.lan), mine)];
  if (others.length) rows.push(element('p', 'faint', say('settings-page/lan-others').replace('{urls}', others.join('、'))));
  if (got.macos) rows.push(element('p', 'faint', say('settings-page/lan-mac-firewall')));
  return rows;
}
