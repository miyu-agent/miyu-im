// 会话的浮窗（施工 I-7 上，设计 01 第八节第 3、7 条）：比小浮窗大，左边是这个会话（头像、名字、号）和几页，右边是这一页。
//
// - 几页照设计：概况、人格和模型、触发方式（只有群聊的两张）、限流与并发、发送消息处理、管理插件。概况（I-7 上）、人格和模型、
//   限流与并发、发送消息处理（I-7 中一，form.js 照 fields.js 画）、触发方式（I-7 中二）、管理插件（I-7 中四，plugins.js）做好了；
//   一格都没有的页不列。
// - 概况：是否回复（私聊白名单里的人没有这一格：在名单里就是要回），改了当场存（cards.save_one），存好了宿主推 cards、墙跟着变。
//   下面这个会话的统计（施工 I-101，venue-stats.js：今天、7 天、30 天，宿主推了变了隔 30 秒才再问）和最近判断（施工 I-7 上补，
//   overview.js）：开的时候问一次 cards.overview，宿主推了变了（来了消息、卡片变了）再问。
//   群聊名单、私聊白名单里的卡底下有「移出」：点一次换成「再点一次移出」，再点才移（cards.remove，私聊的还移出白名单），移了关浮窗。
// - 「插件加的」（施工 I-8 下，设计 01 第四节第 5 条）：声明了浮窗一页、在这个会话开着的插件（2026-10-20 项目主人定：开着才出现）
//   一个一页，兜底卡上没有；内容经插件登记的 chat.panel 取（plugins/widget-host.js），这一个浮窗开着时只取一次，点了动作、存了表单再取。
// - 「定时任务」（设计 12，施工 I-198，tasks.js）：名单里的群、私聊卡上有，排在「触发方式」下一行。
// - 「实时日志」（设计 10，施工 I-161，watch.js）：名单里的群、私聊卡上有，切到这一页才开始看，离开、关浮窗就停。
// - 「会话默认值」那张卡（2026-10-20 项目主人验收：从设置页挪到会话页墙上第一张）也开这个浮窗：没有概况、管理插件，从人格和模型起。
import { element, say } from '../../shell/i18n.js';
import { sect } from '../../shell/sect.js';
import { closeButton, openLayer, toast } from '../../shell/dialog.js';
import { faceOf } from './face.js';
import { cardOf, isDefaults, ownCardOf, valueFor } from './model.js';
import { venueKinds } from '../home/platforms.js';
import { idLine, idWord as idWordOf, titleOf } from './names.js';
import { drawOverview, hearingMarks } from './overview.js';
import { venueStats } from './venue-stats.js';
import { FIELDS, has } from './fields.js';
import { drawForm } from './form.js';
import { drawPlugins, fits, isOn, named as pick } from './plugins.js';
import { mountWidget } from '../plugins/widget-host.js';
import { goTo } from '../../shell/connection.js';
import { watchVenue } from './watch.js';
import { taskPage } from './tasks.js';

/** 照 `kinds` 列的几页：编号、做好了没有、只在哪几种卡上有。 */
const TABS = [
  { id: 'overview', kinds: ['group', 'other_groups', 'private', 'other_privates'], ready: true },
  { id: 'persona', ready: true },
  // 私聊卡也有「触发方式」（施工 I-111，项目主人 11-02：私聊白名单的卡上找不到「时不时看一眼」）：只有看一眼那一格。
  { id: 'trigger', kinds: ['group', 'other_groups', 'private', 'defaults', 'private_defaults'], ready: true },
  // 定时任务（设计 12，施工 I-198）：排在「触发方式」下一行；名单里一个群、一个私聊的卡上才有，任务是给一个具体的会话的。
  { id: 'tasks', kinds: ['group', 'private'], ready: true },
  { id: 'pace', ready: true },
  { id: 'send', ready: true },
  { id: 'plugins', kinds: ['group', 'other_groups', 'private', 'other_privates'], ready: true },
  // 实时日志（设计 10，施工 I-161）：名单里一个群、一个私聊的卡上才有，看它里面 AI 在干嘛。
  { id: 'watch', kinds: ['group', 'private'], ready: true },
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
  // 实时日志那一页开着的：跨重画留着，离开那一页、关浮窗时停。
  let watching = null;
  const unwatch = () => {
    watching?.stop();
    watching = null;
  };
  const close = openLayer(dialog, () => {
    unwatch();
    onClose?.();
  });
  head.append(title, closeButton(close));
  let tab = isDefaults(one) ? 'persona' : 'overview';
  // 统计和最近判断：重画浮窗时留着这两块，问到了再换。
  const counted = isDefaults(one) ? null : venueStats(context.request, () => cardOf(shown));
  const today = element('div', 'venue-today');
  // 概况标题右边的小标签（Telegram 的群开着隐私模式收不全、话题群，施工 I-183 下）：问 cards.overview 时记下，画标题时照它。
  let hearing = null;
  let asking = 0;
  // 最近判断的列表头搜了什么、筛了什么（施工 I-166）：浮窗开着时一直留着。
  const recentState = {};
  // 定时任务那一页（施工 I-198）：头一回切过去才问宿主，跨重画留着（正在填的不丢）。
  let tasking = null;
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
    if (isDefaults(shown)) return;
    counted.refresh();
    const mine = ++asking;
    try {
      const answer = await context.request('cards.overview', { card: cardOf(shown) });
      if (mine === asking) {
        hearing = answer.hearing || null;
        if (tab === 'overview') title.replaceChildren(say('venue/tab-overview'), ...hearingMarks(hearing));
        drawOverview(today, answer, shown.type, recentState);
      }
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
    if (tab.startsWith('plugin:') && !plugin) tab = isDefaults(shown) ? 'persona' : 'overview';
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
    if (tab !== 'watch') unwatch();
    title.textContent = plugin ? pick(plugin.name) || plugin.package : say(`venue/tab-${tab}`);
    if (!plugin && tab === 'overview') title.append(...hearingMarks(hearing));
    if (plugin) body.replaceChildren(panelOf(plugin));
    else if (tab === 'overview') body.replaceChildren(...overview(shown, context, close, [counted.box, today]));
    else if (tab === 'plugins') body.replaceChildren(...drawPlugins(shown, context));
    else if (tab === 'tasks') {
      tasking ??= taskPage(() => cardOf(shown), context.request);
      body.replaceChildren(tasking.box);
    } else if (tab === 'watch') {
      watching ??= watchVenue(`${context.platform}:${shown.type}:${shown.id}`, context.request);
      body.replaceChildren(watching.box);
    } else {
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

/** 概况那一页：`kept` 是统计和最近判断那两块。 */
function overview(one, { request, platform }, close, kept) {
  const parts = [];
  // 是否回复一张没名字的卡（设计 11 G1，施工 I-166），统计、看一眼、最近判断各一张，「移出」在最底下的「危险操作」卡里。
  const top = sect(null);
  const reply = element('div', 'row');
  const label = element('div', 'label');
  if (one.on === null) {
    const hint = venueKinds(platform).private?.listedHint() || say('venue/listed-hint');
    label.append(element('div', 'label-name', say('venue/listed')), element('small', null, hint));
    reply.append(label);
  } else {
    label.append(element('div', 'label-name', say('venue/reply')), element('small', null, say('venue/reply-hint')));
    reply.append(label, toggle(one, request));
  }
  top.body.append(reply);
  parts.push(top.box, ...kept);
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

/** 「移出」：在「危险操作」卡片里，点两次才移。 */
function removal(one, request, close) {
  const danger = sect(say('venue/danger'));
  const foot = element('div', 'venue-foot');
  danger.body.append(foot);
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
  foot.append(element('span', 'faint', say(`venue/remove-${one.kind}-hint`)), button);
  return danger.box;
}
