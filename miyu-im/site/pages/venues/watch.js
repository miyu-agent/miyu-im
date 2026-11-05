// 会话浮窗的「实时日志」（设计 10，施工 I-161）：这个会话里 AI 在干嘛，一行一件事，思考、正文边写边出。
//
// - 开这一页时 watch.start（短编号 `<平台>:<group|private>:<号>`），先画宿主留着的最近一段，再接推来的；离开这一页、关浮窗时
//   watch.stop。通道断了重连上，再开始看一次、整个重画。
// - 字照 texts.js 的 `watch/…`（和命令行字表里的同名），时刻照本机的钟。
// - 写着的块（思考、正文、调工具）照编号接在那一行后面，写完换成定下来的字。
// - 贴着底部时来了新的跟着滚；往上翻着的不动。最多留 500 行。
import { element, say } from '../../shell/i18n.js';
import { listenWatch } from '../../shell/watch.js';

/** 最多留几行。 */
const MOST = 500;

/** 有说法的词照字表（`watch/<group>-<词>`），没有的照原样。 */
function word(group, value) {
  const key = `watch/${group}-${value}`;
  const said = say(key);
  return said === key ? value : said;
}

/** 本机的钟点 `HH:MM:SS`。 */
export function clock(at) {
  const date = new Date(at);
  return [date.getHours(), date.getMinutes(), date.getSeconds()].map((n) => String(n).padStart(2, '0')).join(':');
}

/** 一行说成话（纯逻辑，照命令行 `watch/render.rs`）：{ kind, text, id?, done }。 */
export function spoken(item) {
  switch (item.kind) {
    case 'heard': {
      const extras = (item.extras || []).map((extra) => {
        if (extra.kind === 'images') return say('watch/images', { count: extra.value });
        if (extra.kind === 'sticker') return say('watch/sticker');
        if (extra.kind === 'forward') return say('watch/forward', { count: extra.value });
        return say('watch/other', { kind: extra.value });
      });
      const parts = item.text ? [item.text, ...extras] : extras;
      return { kind: 'heard', text: say('watch/heard', { who: item.who, text: parts.join(' ') }) };
    }
    case 'decided': {
      const parts = [word('outcome', item.outcome)];
      parts.push(item.conditions?.length ? item.conditions.map((one) => word('condition', one)).join(say('watch/separator')) : say('watch/no-condition'));
      if (item.score) parts.push(say('watch/score', { total: item.score.total.toFixed(2), threshold: item.score.threshold.toFixed(2) }));
      if (item.judge) {
        const seconds = item.judge.millis == null ? '' : (item.judge.millis / 1000).toFixed(1);
        if (item.judge.unjudged === 'timeout') parts.push(say('watch/judge-timeout', { seconds }));
        else if (item.judge.unjudged) parts.push(say('watch/judge-not', { why: word('why', item.judge.unjudged) }));
        else parts.push(say('watch/judge', { seconds, reason: item.judge.reason || '' }));
      }
      if (item.why) parts.push(word('why', item.why));
      return { kind: 'decided', text: say('watch/decided', { who: item.who || '', text: parts.join(' · ') }) };
    }
    case 'started':
      if (item.glance) return { kind: 'started', text: say('watch/started-glance') };
      return { kind: 'started', text: item.who ? say('watch/started', { who: item.who }) : say('watch/started-anyone') };
    case 'wrote': {
      const block = item.block?.kind;
      const base = { id: item.id, done: item.done };
      if (block === 'thinking') return { ...base, kind: 'thinking', text: say('watch/thinking', { text: item.text }) };
      if (block === 'call') return { ...base, kind: 'call', text: say('watch/call', { tool: item.block.tool, args: item.text }) };
      return { ...base, kind: 'said', text: say('watch/said', { text: item.text }) };
    }
    case 'appended':
      return { kind: 'append', id: item.id, text: item.text, done: false };
    case 'result':
      return { kind: 'result', text: say(item.ok ? 'watch/result' : 'watch/result-error', { tool: item.tool, summary: item.summary }) };
    case 'delivered':
      if (!item.text && item.images) return { kind: 'delivered', text: say('watch/delivered-images', { count: item.images }) };
      return { kind: 'delivered', text: say('watch/delivered', { text: item.text }) };
    case 'failed':
      return { kind: 'failed', text: say('watch/failed', { why: word('failed-why', item.why) }) };
    case 'recalled':
      return { kind: 'recalled', text: say('watch/recalled', { who: item.who }) };
    case 'compacted':
      return { kind: 'compacted', text: say('watch/compacted') };
    case 'ended':
      return { kind: 'ended', text: say('watch/ended', { reason: word('ended-why', item.reason) }) };
    default:
      return null;
  }
}

/** 画「实时日志」这一页：会话的短编号 `venue`，经 `request` 调宿主。交回 { box, stop() }。 */
export function watchVenue(venue, request) {
  const box = element('div', 'watch');
  const list = element('div', 'watch-lines');
  const empty = element('p', 'faint watch-empty', say('watch/empty'));
  box.append(list, empty);
  const blocks = new Map();
  let stopped = false;

  const stuck = () => list.scrollHeight - list.scrollTop - list.clientHeight < 24;
  const add = (line) => {
    const said = spoken(line.item);
    if (!said) return;
    if (said.kind === 'append') {
      const known = blocks.get(said.id);
      if (known) known.text.append(said.text);
      return;
    }
    const known = said.id ? blocks.get(said.id) : null;
    if (known) {
      known.text.textContent = said.text;
      known.row.classList.toggle('writing', !said.done);
      return;
    }
    const row = element('div', `watch-line is-${said.kind}${said.done === false ? ' writing' : ''}`);
    const text = element('span', 'watch-text', said.text);
    row.append(element('span', 'watch-at', clock(line.at)), text);
    list.append(row);
    if (said.id) blocks.set(said.id, { row, text });
    while (list.childElementCount > MOST) list.firstElementChild.remove();
  };
  const take = (lines) => {
    const follow = stuck();
    for (const line of lines) add(line);
    empty.hidden = list.childElementCount > 0;
    if (follow) list.scrollTop = list.scrollHeight;
  };
  const start = async () => {
    try {
      const answer = await request('watch.start', { venue });
      if (stopped) return;
      list.replaceChildren();
      blocks.clear();
      take((answer.batches || []).flatMap((batch) => batch.lines));
    } catch {
      // 通道断了：重连上以后 onReopen 再开始。
    }
  };
  const forget = listenWatch(
    (message) => {
      if (message.reset) {
        list.replaceChildren();
        blocks.clear();
        take((message.batches || []).flatMap((batch) => batch.lines));
      } else if (message.batch?.venue === venue) {
        take(message.batch.lines);
      }
    },
    () => start(),
  );
  start();
  return {
    box,
    stop() {
      if (stopped) return;
      stopped = true;
      forget();
      request('watch.stop').catch(() => {});
    },
  };
}
