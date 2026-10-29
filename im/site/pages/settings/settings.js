// 设置页（施工 I-7 下一，设计 01 第八节第 6 条）：只放 im 自己的、各平台通用的设置。几段：本人的平台 ID（核心的 `external.bindings`，
// 施工 I-68）、通用（回执自动撤回的命令）、网页服务（端口；登录用访问密码）。照样板左边一列切段、右边一次一段
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

/** 在 `view` 里起设置页：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountSettings(view, { request }) {
  // 外壳问到的连接；加身份时选着的平台（没选过的照首页选中的连接）。
  let connections = [];
  let adding = '';
  // 正开着的一段。
  let page = 'bindings';
  // 问到的两样：连接变了照它重画，不再问。
  let got = null;
  let words = null;
  const draw = () => {
    if (!got) return;
    const platforms = [...new Set(connections.map((one) => one.platform))];
    if (!platforms.includes(adding)) adding = chosenOf(connections)?.platform || platforms[0] || '';
    const choose = (next) => {
      adding = next;
      draw();
    };
    const pages = [
      ['bindings', bindings(got, { platforms, adding, choose }, save)],
      ['general', general(got, save)],
      ...(words ? [['moderation', moderation(words, request, load)]] : []),
      ['site', site(got, save)],
    ];
    view.replaceChildren(head(), tabbed(pages, page, (key) => (page = key)));
  };
  const load = async () => {
    try {
      [got, words] = await Promise.all([request('settings.get'), request('moderation.get').catch(() => null)]);
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

/** 一段：标题、下面的几行。 */
function section(title, hint, rows) {
  const box = element('div', 'panel settings-section');
  const named = element('div', 'section-title', say(title));
  if (hint) named.append(element('small', null, say(hint)));
  box.append(named, ...rows);
  return box;
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
 *  （施工 I-85 补）每行写平台，加的那一行先选平台：`platforms` 是挂着的平台，`adding` 是选着的，换了调 `choose`。 */
function bindings(got, { platforms, adding, choose }, save) {
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
  // 输入框的提示只写「ID」，不写具体的号（项目主人 10-27）。
  const number = element('input', 'input num wide-num');
  number.placeholder = say('settings-page/bind-id');
  // QQ 号只有数字；别的平台的号照宿主的规矩（不带空白、冒号，48 个字以内，施工 I-79）。
  const digits = adding === 'qq';
  if (digits) number.inputMode = 'numeric';
  const add = element('button', 'btn', say('settings-page/bind'));
  add.disabled = !adding;
  add.addEventListener('click', () => {
    const typed = number.value.trim();
    const fine = digits ? /^\d+$/.test(typed) : /^[^\s:"]{1,48}$/.test(typed);
    if (!fine) return toast(say(digits ? 'settings-page/bind-bad' : 'settings-page/bind-bad-text'));
    save(`binding:${adding}:${typed}`, true);
  });
  const line = element('span', 'inline');
  if (several) {
    const seg = element('div', 'seg');
    for (const one of platforms) {
      const button = element('button', one === adding ? 'on' : null, platform(one).name());
      button.addEventListener('click', () => choose(one));
      seg.append(button);
    }
    line.append(seg);
  }
  line.append(number, add);
  rows.push(row(say('settings-page/bind-new'), '', line));
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

/** 违规判断（施工 I-16，照旧版设置页）：违规关键词，存成系统那一份（整份替换出厂的），能回到出厂的。门槛在会话浮窗「高级」里。 */
function moderation(words, request, reload) {
  const put = async (keywords) => {
    try {
      const answer = await request('moderation.set', { keywords });
      if (!answer.saved) toast(say('failed/save'));
    } catch {
      toast(say('failed/save'));
      reload();
    }
  };
  const tags = tagInput({ items: words.keywords || [], save: (list) => put(list) });
  const rows = [row(say('settings-page/moderation-words'), say('settings-page/moderation-words-hint'), tags)];
  if (words.own) {
    const back = element('button', 'btn', say('settings-page/moderation-factory'));
    back.addEventListener('click', async () => {
      await put(null);
      reload();
    });
    rows.push(row(say('settings-page/moderation-factory-label'), null, back));
  }
  return section('settings-page/moderation', 'settings-page/moderation-hint', rows);
}

/** 网页服务：端口；登录用 Miyu 账号。 */
function site(got, save) {
  const port = element('input', 'input num');
  port.inputMode = 'numeric';
  port.value = got.port ?? '';
  port.addEventListener('change', () => {
    const number = Number(port.value.trim());
    if (Number.isInteger(number) && number > 0 && number < 65536) save('port', number);
    else toast(say('failed/invalid'));
  });
  return section('settings-page/site', 'settings-page/site-hint', [row(say('settings-page/port'), say('settings-page/port-hint'), port)]);
}
