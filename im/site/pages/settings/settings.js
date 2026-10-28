// 设置页（施工 I-7 下一，设计 01 第八节第 6 条）：只放 im 自己的、各平台通用的设置。几段：账号绑定（平台身份 → Miyu 账号，核心的
// `external.bindings`）、通用（去掉末尾句号、模拟真人一条分多条发送、回执自动撤回的命令）、网页服务（端口；登录用 Miyu 账号）。照样板左边一列切段、右边一次一段
// （施工 I-7 下补）；切到哪一段重画时照旧。会话默认值（施工 I-7 下二）2026-10-20 挪到会话页墙上第一张卡（venues/）。
//
// - 读写都经宿主的 settings.get、settings.set（宿主在这份登录的人的连接上改核心的配置）。改了当场存，存不成的提示一句、重读一遍；
//   回执撤回的命令是标签（shell/tags.js），加了当场画上，存成了不重读（重读会把正在填的那一格换掉），存不成的才重读。
import { element, icon, say, spaced } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { tagInput } from '../../shell/tags.js';
import { venueKinds } from '../home/platforms.js';

/** 在 `view` 里起设置页：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountSettings(view, { request }) {
  let platform = '';
  // 正开着的一段。
  let page = 'bindings';
  const load = async () => {
    let got;
    let words;
    try {
      [got, , words] = await Promise.all([
        request('settings.get'),
        platform ? null : request('status').then((status) => (platform = status.platform || '')),
        request('moderation.get').catch(() => null),
      ]);
    } catch {
      view.replaceChildren(head(), element('p', 'faint', say('settings-page/failed')));
      return;
    }
    const pages = [
      ['bindings', bindings(got, platform, save)],
      ['general', general(got, save)],
      ...(words ? [['moderation', moderation(words, request, load)]] : []),
      ['site', site(got, save)],
    ];
    view.replaceChildren(head(), tabbed(pages, page, (key) => (page = key)));
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
    update() {},
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

/** 账号绑定：一个平台身份一行（号、对应的账号、去掉），最后一行加一个。只列这个平台的身份。 */
function bindings(got, platform, save) {
  const word = venueKinds(platform).private?.id() || say('venues/user-id');
  const prefix = `${platform}:`;
  const rows = [];
  for (const [identity, account] of Object.entries(got.bindings || {})) {
    if (platform && !identity.startsWith(prefix)) continue;
    const number = identity.slice(prefix.length);
    const box = element('span', 'inline');
    const remove = element('button', 'icon-button');
    remove.title = say('settings-page/unbind');
    remove.append(icon('i-x'));
    remove.addEventListener('click', () => save(`binding:${identity}`, null));
    box.append(element('b', 'binding-account', String(account)), remove);
    rows.push(row(spaced(`${word} ${number}`), '', box));
  }
  // 输入框的提示只写「ID」「账号名」，不写具体的号、账号（项目主人 10-27）。
  const number = element('input', 'input num wide-num');
  number.placeholder = say('settings-page/bind-id');
  number.inputMode = 'numeric';
  const account = element('input', 'input');
  account.placeholder = say('settings-page/bind-account');
  const add = element('button', 'btn', say('settings-page/bind'));
  add.addEventListener('click', () => {
    const typed = number.value.trim();
    if (!/^\d+$/.test(typed)) return toast(say('settings-page/bind-bad'));
    // 核心 SU-1 起只有主人：对应表里不空的值都当主人，没填的照已经绑了的那个写，一个都没绑的写 owner。
    save(`binding:${prefix}${typed}`, account.value.trim() || Object.values(got.bindings || {})[0] || 'owner');
  });
  const adding = element('span', 'inline');
  adding.append(number, account, add);
  rows.push(row(say('settings-page/bind-new'), '', adding));
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
