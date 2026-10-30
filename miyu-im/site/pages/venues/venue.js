// 会话的浮窗（施工 I-7 上，设计 01 第八节第 3、7 条）：比小浮窗大，左边是这个会话（头像、名字、号）和几页，右边是这一页。
//
// - 几页照设计：概况、人格和模型、触发方式（只有群聊的两张）、限流与并发、发送消息处理、管理插件。概况（I-7 上）、人格和模型、
//   限流与并发、发送消息处理（I-7 中一，form.js 照 fields.js 画）、触发方式（I-7 中二）、管理插件（I-7 中四，plugins.js）做好了；
//   一格都没有的页不列。
// - 概况：是否回复（私聊白名单里的人没有这一格：在名单里就是要回），改了当场存（cards.save_one），存好了宿主推 cards、墙跟着变。
//   下面今天的数和最近判断（施工 I-7 上补，overview.js）：开的时候问一次 cards.overview，宿主推了变了（来了消息、卡片变了）再问。
//   群聊名单、私聊白名单里的卡底下有「移出」：点一次换成「再点一次移出」，再点才移（cards.remove，私聊的还移出白名单），移了关浮窗。
// - 「插件加的」（施工 I-8 下，设计 01 第四节第 5 条）：声明了浮窗一页、在这个会话开着的插件（2026-10-20 项目主人定：开着才出现）
//   一个一页，兜底卡上没有；内容经插件登记的 chat.panel 取（plugins/widget-host.js），这一个浮窗开着时只取一次，点了动作、存了表单再取。
// - 「会话默认值」那张卡（2026-10-20 项目主人验收：从设置页挪到会话页墙上第一张）也开这个浮窗：没有概况、管理插件，从人格和模型起。
import { element, say } from '../../shell/i18n.js';
import { closeButton, openLayer, toast } from '../../shell/dialog.js';
import { faceOf } from './face.js';
import { cardOf, ownCardOf, valueFor } from './model.js';
import { venueKinds } from '../home/platforms.js';
import { idLine, idWord as idWordOf, titleOf } from './names.js';
import { drawOverview } from './overview.js';
import { FIELDS, has } from './fields.js';
import { drawForm } from './form.js';
import { drawPlugins, fits, isOn, named as pick } from './plugins.js';
import { mountWidget } from '../plugins/widget-host.js';
import { goTo } from '../../shell/connection.js';

/** 照 `kinds` 列的几页：编号、做好了没有、只在哪几种卡上有。 */
const TABS = [
  { id: 'overview', kinds: ['group', 'other_groups', 'private', 'other_privates'], ready: true },
  { id: 'persona', ready: true },
  { id: 'trigger', kinds: ['group', 'other_groups', 'defaults'], ready: true },
  { id: 'pace', ready: true },
  { id: 'send', ready: true },
  { id: 'plugins', kinds: ['group', 'other_groups', 'private', 'other_privates'], ready: true },
];

/** 开 `one` 的浮窗；`context` 是 { request, platform, bots, personas }（bots 是机器人的号到名字，连着不止一个时卡上写是哪个；
 *  personas 是人格的 [{persona, name}]），关了调
 *  `onClose`。交回 { update(one), refresh(), close() }：宿主推了卡片变了，会话页拿新的那一张调 update；来了消息调 refresh 再问
 *  今天的数和最近判断。 */
export function openVenue(one, context, onClose) {
  let shown = one;
  const dialog = element('div', 'dialog wide');
  const side = element('div', 'd-side');
  const main = element('div', 'd-main');
  const head = element('div', 'd-head');
  const title = element('h2');
  const body = element('div', 'd-body');
  main.append(head, body);
  dialog.append(side, main);
  const close = openLayer(dialog, onClose);
  head.append(title, closeButton(close));
  let tab = one.kind === 'defaults' ? 'persona' : 'overview';
  // 今天的数和最近判断：重画浮窗时留着这一块，问到了再换。
  const today = element('div', 'venue-today');
  let asking = 0;
  // 哪几页的「高级」展开着：重画时照旧。
  const expanded = new Set();
  // 插件加的几页：编号 → 画好的那一块（只取一次）。
  const panels = new Map();
  const panelOf = (plugin) => {
    if (!panels.has(plugin.package)) {
      const box = element('div', 'plugin-panel');
      mountWidget(box, plugin.package, { place: 'panel', venue: { type: shown.type, id: shown.id } }, {
        request: context.request,
        openPage: (id) => {
          close();
          goTo(`plugins/${encodeURIComponent(plugin.package)}/page/${encodeURIComponent(id)}`);
        },
      });
      panels.set(plugin.package, box);
    }
    return panels.get(plugin.package);
  };
  // 选模型的下拉：开的时候问一次宿主（models），问到了重画这一页。
  let models = null;
  context
    .request('models')
    .then((answer) => {
      models = answer;
      if (tab === 'persona') draw();
    })
    .catch(() => {});
  // 能贴的表情：开的时候问一次宿主（reactions），问到了重画这一页。
  let reactions = null;
  context
    .request('reactions')
    .then((answer) => {
      reactions = answer;
      if (tab === 'send') draw();
    })
    .catch(() => {});
  // 预设：开的时候问一次宿主（presets），问不到的（宿主还没有这个方法）那一格不画。
  let presets = null;
  context
    .request('presets')
    .then((answer) => {
      presets = answer;
      if (tab === 'persona') draw();
    })
    .catch(() => {});
  const refresh = async () => {
    if (shown.kind === 'defaults') return;
    const mine = ++asking;
    try {
      const answer = await context.request('cards.overview', { card: cardOf(shown) });
      if (mine === asking) drawOverview(today, answer, shown.type);
    } catch {
      // 问不到的：照旧画上一次的。
    }
  };

  const draw = () => {
    const who = element('div', 'd-who');
    const named = element('div');
    named.append(element('b', null, titleOf(shown)), element('small', null, idLine(shown, context)));
    who.append(faceOf(shown), named);
    const tabs = element('div', 'd-tabs');
    side.replaceChildren(who, tabs);
    const added = shown.id
      ? (context.plugins || []).filter((plugin) => plugin.ui?.panel && fits(shown, plugin) && isOn(shown, plugin, context.fallbacks?.[shown.type] || {}))
      : [];
    const plugin = added.find((one) => `plugin:${one.package}` === tab);
    if (tab.startsWith('plugin:') && !plugin) tab = shown.kind === 'defaults' ? 'persona' : 'overview';
    for (const one of TABS) {
      if (one.kinds && !one.kinds.includes(shown.kind)) continue;
      const form = one.id === 'overview' ? null : FIELDS[one.id];
      if (one.ready && form && !form.some((field) => field.key && has(field, shown))) continue;
      const button = element('button', one.id === tab ? 'd-tab on' : 'd-tab', say(`venue/tab-${one.id}`));
      if (!one.ready) {
        button.disabled = true;
        button.append(element('span', 'soon', say('nav/soon')));
      } else {
        button.addEventListener('click', () => {
          tab = one.id;
          draw();
        });
      }
      tabs.append(button);
    }
    if (added.length > 0) tabs.append(element('div', 'd-sep', say('venue/plugins-added')));
    for (const one of added) {
      const id = `plugin:${one.package}`;
      const button = element('button', id === tab ? 'd-tab on' : 'd-tab', pick(one.name) || one.package);
      button.addEventListener('click', () => {
        tab = id;
        draw();
      });
      tabs.append(button);
    }
    title.textContent = plugin ? pick(plugin.name) || plugin.package : say(`venue/tab-${tab}`);
    if (plugin) body.replaceChildren(panelOf(plugin));
    else if (tab === 'overview') body.replaceChildren(...overview(shown, context, close, today));
    else if (tab === 'plugins') body.replaceChildren(...drawPlugins(shown, context));
    else {
      const idWord = idWordOf('private', context.platform);
      body.replaceChildren(...drawForm(tab, shown, { ...context, models, reactions, presets, expanded, idWord }));
    }
  };
  draw();
  refresh();
  return {
    update(now) {
      shown = now;
      // 正在填的那一格别被重画掉：它存的时候宿主还会再推一次。
      const typing = body.contains(document.activeElement) && document.activeElement.matches('input, textarea, select');
      if (!typing) draw();
      refresh();
    },
    refresh,
    close,
  };
}

/** 概况那一页。 */
function overview(one, { request, platform }, close, today) {
  const parts = [];
  const reply = element('div', 'row big');
  const label = element('div', 'label');
  if (one.on === null) {
    const hint = venueKinds(platform).private?.listedHint() || say('venue/listed-hint');
    label.append(element('b', null, say('venue/listed')), element('small', null, hint));
    reply.append(label);
  } else {
    label.append(element('b', null, say('venue/reply')), element('small', null, say('venue/reply-hint')));
    reply.append(label, toggle(one, request));
  }
  parts.push(reply, today);
  if (one.id) parts.push(removal(one, request, close));
  return parts;
}

/** 是否回复的开关：点了当场存。 */
export function toggle(one, request) {
  const button = element('button', one.on ? 'switch on' : 'switch');
  button.append(element('i'));
  button.title = say('venue/reply');
  button.addEventListener('click', async (event) => {
    event.stopPropagation();
    const next = !button.classList.contains('on');
    button.classList.toggle('on', next);
    button.closest('.vcard')?.classList.toggle('off', !next);
    const saved = await save(request, one, { on: valueFor(one, 'on', next) });
    if (!saved) {
      button.classList.toggle('on', !next);
      button.closest('.vcard')?.classList.toggle('off', next);
    }
  });
  return button;
}

/** 存 `one` 的几格：成了交回真，不成的提示一句。 */
async function save(request, one, values) {
  try {
    const answer = await request('cards.save_one', { card: cardOf(one), values });
    if (answer.saved) return true;
    toast(answer.message || answer.problems?.[0]?.message || say('failed/save'));
  } catch {
    toast(say('failed/save'));
  }
  return false;
}

/** 「移出」：点两次才移。 */
function removal(one, request, close) {
  const foot = element('div', 'venue-foot');
  const button = element('button', 'btn ghost danger', say(`venue/remove-${one.kind}`));
  let armed = false;
  button.addEventListener('click', async () => {
    if (!armed) {
      armed = true;
      button.textContent = say('venue/remove-again');
      return;
    }
    button.disabled = true;
    try {
      const answer = await request('cards.remove', { card: ownCardOf(one) });
      if (answer.saved) {
        close();
        return;
      }
      toast(answer.message || say('failed/save'));
    } catch {
      toast(say('failed/save'));
    }
    button.disabled = false;
  });
  foot.append(button);
  return foot;
}
