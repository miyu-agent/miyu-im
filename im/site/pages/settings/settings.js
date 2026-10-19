// 设置页（施工 I-7 下一，设计 01 第八节第 6 条）：只放 im 自己的、各平台通用的设置。几段：会话默认值（施工 I-7 下二）、账号绑定（平台身份
// → Miyu 账号，核心的 `external.bindings`）、通用（去掉末尾句号、回执自动撤回的命令）、网页服务（端口；登录用 Miyu 账号）。
//
// - 会话默认值是一张卡（`cards.json` 的 `defaults`，项目主人 10-20 定）：照会话浮窗那几页同一套格画（form.js），卡上没改的格照它。存照
//   cards.save_one，宿主推 cards 再读；正在填的那一格不被重画掉。
// - 别的读写都经宿主的 settings.get、settings.set（宿主在这份登录的人的连接上改核心的配置）。改了当场存，存不成的提示一句、重读一遍。
import { element, icon, say, spaced } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { venueKinds } from '../home/platforms.js';
import { drawForm } from '../venues/form.js';
import { idWord } from '../venues/names.js';

/** 会话默认值画哪几页（照会话浮窗的先后，管理插件不在这里）。 */
const TABS = ['persona', 'trigger', 'pace', 'send'];

/** 在 `view` 里起设置页：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountSettings(view, { request }) {
  let platform = '';
  // 会话默认值要的：模型、能贴的表情问一次；「高级」展开着的重画时照旧。
  let models = null;
  let reactions = null;
  const expanded = new Set();
  const load = async () => {
    let got;
    let cards;
    try {
      [got, cards] = await Promise.all([
        request('settings.get'),
        request('cards.get'),
        platform ? null : request('status').then((status) => (platform = status.platform || '')),
        models ? null : request('models').then((answer) => (models = answer), () => null),
        reactions ? null : request('reactions').then((answer) => (reactions = answer), () => null),
      ]);
    } catch {
      view.replaceChildren(head(), element('p', 'faint', say('settings-page/failed')));
      return;
    }
    const context = { request, platform, models, reactions, expanded, personas: cards.personas || [], friends: cards.friends || [], idWord: idWord('private', platform) };
    view.replaceChildren(head(), defaults(cards, context), ...bindings(got, platform, save), ...general(got, save), ...site(got, save));
  };
  const save = async (key, value) => {
    try {
      const answer = await request('settings.set', { key, value });
      if (!answer.saved) toast(say('failed/save'));
    } catch {
      toast(say('failed/save'));
    }
    load();
  };
  load();
  return {
    update() {},
    event(name) {
      // 卡片变了（会话默认值存了）：再读一遍；正在填的那一格不被重画掉，它存的时候宿主还会再推一次。
      const typing = view.contains(document.activeElement) && document.activeElement.matches('input, textarea, select');
      if (name === 'cards' && !typing) load();
    },
    dispose() {},
  };
}

/** 会话默认值：照会话浮窗那几页的格画（form.js），一页一小段。 */
function defaults(cards, context) {
  const one = {
    kind: 'defaults',
    type: 'group',
    id: null,
    name: '',
    members: null,
    bot: null,
    set: cards.cards?.defaults || {},
    defaults: cards.defaults?.defaults || {},
    on: null,
  };
  const rows = [];
  for (const tab of TABS) {
    const form = drawForm(tab, one, context);
    if (form.length === 0) continue;
    rows.push(element('div', 'group-name', say(`venue/tab-${tab}`)), ...form);
  }
  return section('settings-page/defaults', 'settings-page/defaults-hint', rows);
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
  const number = element('input', 'input num wide-num');
  number.placeholder = word;
  number.inputMode = 'numeric';
  const account = element('input', 'input');
  account.placeholder = got.account || '';
  const add = element('button', 'btn', say('settings-page/bind'));
  add.addEventListener('click', () => {
    const typed = number.value.trim();
    if (!/^\d+$/.test(typed)) return toast(say('settings-page/bind-bad'));
    save(`binding:${prefix}${typed}`, account.value.trim() || got.account);
  });
  const adding = element('span', 'inline');
  adding.append(number, account, add);
  rows.push(row(say('settings-page/bind-new'), '', adding));
  return [section('settings-page/bindings', 'settings-page/bindings-hint', rows)];
}

/** 通用：去掉末尾句号、回执自动撤回的命令。 */
function general(got, save) {
  const strip = element('button', got.strip_period ? 'switch on' : 'switch');
  strip.append(element('i'));
  strip.addEventListener('click', () => save('strip_period', !got.strip_period));
  const commands = got.recall_receipts || [];
  const tags = element('div', 'tags');
  commands.forEach((command, at) => {
    const tag = element('span', 'tag', command);
    const remove = element('button');
    remove.title = say('field/remove');
    remove.append(icon('i-x'));
    remove.addEventListener('click', () => save('recall_receipts', commands.filter((_, index) => index !== at)));
    tag.append(remove);
    tags.append(tag);
  });
  const adding = element('input', 'input tag-add');
  adding.placeholder = say('field/add-word');
  adding.addEventListener('keydown', (event) => {
    const typed = adding.value.trim().replace(/^\//, '');
    if (event.key === 'Enter' && typed) save('recall_receipts', [...commands, typed]);
  });
  tags.append(adding);
  return [
    section('settings-page/general', null, [
      row(say('settings-page/strip-period'), say('settings-page/strip-period-hint'), strip),
      row(say('settings-page/recall'), say('settings-page/recall-hint'), tags),
    ]),
  ];
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
  return [
    section('settings-page/site', 'settings-page/site-hint', [row(say('settings-page/port'), say('settings-page/port-hint'), port)]),
  ];
}
