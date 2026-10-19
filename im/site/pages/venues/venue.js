// 会话的浮窗（施工 I-7 上，设计 01 第八节第 3、7 条）：比小浮窗大，左边是这个会话（头像、名字、号）和几页，右边是这一页。
//
// - 几页照设计：概况、人格和模型、触发方式（只有群聊的两张）、限流与并发、发送消息处理、管理插件。这一步只做了概况，别的灰着写「即将」。
// - 概况：是否回复（私聊白名单里的人没有这一格：在名单里就是要回），改了当场存（cards.save_one），存好了宿主推 cards、墙跟着变。
//   下面今天的数和最近判断（施工 I-7 上补，overview.js）：开的时候问一次 cards.overview，宿主推了变了（来了消息、卡片变了）再问。
//   群聊名单、私聊白名单里的卡底下有「移出」：点一次换成「再点一次移出」，再点才移（cards.remove，私聊的还移出白名单），移了关浮窗。
import { element, say } from '../../shell/i18n.js';
import { closeButton, openLayer, toast } from '../../shell/dialog.js';
import { face } from './face.js';
import { cardOf, valueFor } from './model.js';
import { venueKinds } from '../home/platforms.js';
import { idLine, titleOf } from './names.js';
import { drawOverview } from './overview.js';

/** 照 `kinds` 列的几页：编号、做好了没有、只在哪几种卡上有。 */
const TABS = [
  { id: 'overview', ready: true },
  { id: 'persona' },
  { id: 'trigger', kinds: ['group', 'other_groups'] },
  { id: 'pace' },
  { id: 'send' },
  { id: 'plugins' },
];

/** 开 `one` 的浮窗；`context` 是 { request, platform, bots }（bots 是机器人的号到名字，连着不止一个时卡上写是哪个），关了调
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
  let tab = 'overview';
  // 今天的数和最近判断：重画浮窗时留着这一块，问到了再换。
  const today = element('div', 'venue-today');
  let asking = 0;
  const refresh = async () => {
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
    who.append(face(shown.type, shown.id, shown.name), named);
    const tabs = element('div', 'd-tabs');
    side.replaceChildren(who, tabs);
    for (const one of TABS) {
      if (one.kinds && !one.kinds.includes(shown.kind)) continue;
      const button = element('button', one.id === tab ? 'd-tab on' : 'd-tab', say(`venue/tab-${one.id}`));
      if (!one.ready) {
        button.disabled = true;
        button.append(element('span', 'soon', say('nav/soon')));
      }
      tabs.append(button);
    }
    title.textContent = say(`venue/tab-${tab}`);
    body.replaceChildren(...overview(shown, context, close, today));
  };
  draw();
  refresh();
  return {
    update(now) {
      shown = now;
      draw();
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
      const answer = await request('cards.remove', { card: cardOf(one) });
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
