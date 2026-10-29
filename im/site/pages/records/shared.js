// 消息记录页几处共用的（施工 C-1 中下）：去哪个场所、限频、拆场所、写时刻、认人。
import { goTo } from '../../shell/connection.js';

/** 去场所 `venue` 的记录；`id` 是哪一条（行号），空的看最新的。 */
export function openVenue(venue, id) {
  goTo(`records/${encodeURIComponent(venue)}${id ? `/${id}` : ''}`);
}

/** `wait` 毫秒以内最多跑一次 `run`：期间叫过的，到点补跑一次。交回叫它的那一个和停下的那一个。 */
export function throttled(wait, run) {
  let last = 0;
  let timer = 0;
  const call = () => {
    const now = Date.now();
    if (now - last >= wait) {
      last = now;
      run();
    } else if (!timer) {
      timer = setTimeout(() => {
        timer = 0;
        last = Date.now();
        run();
      }, wait - (now - last));
    }
  };
  return { call, stop: () => clearTimeout(timer) };
}

/** 场所 `qq:group:31` 拆成 { platform, type, id }（type 是 group、private）。 */
export function venueParts(venue) {
  // 从右边拆（施工 I-85 中）：号里没有 `:`（施工 I-79），最后一段是号、倒数第二段是种类、头一段是平台，中间的是机器人号（I-81 起场所
  // 编号带它，以前的没有）。
  const parts = String(venue).split(':');
  const platform = parts[0];
  const id = parts.length > 2 ? parts[parts.length - 1] : '';
  const kind = parts.length > 2 ? parts[parts.length - 2] : '';
  const bot = parts.length > 3 ? parts.slice(1, -2).join(':') : '';
  return { platform, type: kind === 'group' ? 'group' : 'private', id, bot };
}

/** 时刻（毫秒）写成本地的月日时分。 */
export function when(at) {
  const date = new Date(at);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleString(undefined, { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

/**
 * 认人：`bots` 是机器人号 → 名字（外壳的连接），`names` 是平台身份 → 读到过的名字。交回 who(sender, name)：她的写机器人的名字，
 * 别人的写这一条带的名字、读到过的名字，都没有的写号。
 */
export function namer(bots, names) {
  return (sender, name) => {
    const number = String(sender || '').split(':').pop();
    if (bots.has(number)) return bots.get(number) || number;
    return name || names.get(sender) || number;
  };
}

/** 外壳的连接 `connections` 里机器人号 → 名字。 */
export function botNames(connections) {
  return new Map((connections || []).map((one) => [String(one.bot), one.name || '']));
}
