// 会话的头像（施工 I-7 上）：群是圆角方块，人是圆的。先画名字的第一个字（照号挑一种底色），页面服务转来的头像（/avatar/group/<群号>、
// /avatar/user/<号>）到了再换上；兜底卡画图标。右下角是平台的角标。
import { element, icon } from '../../shell/i18n.js';
import { mark, platform } from '../home/platforms.js';

const HUES = 5;

/** `one`（model.js 的一张卡）的头像，`id` 是平台的编号（没有的不画角标）。 */
export function face(one, id) {
  const holder = element('span', 'bot');
  const round = element('span', `avatar${one.type === 'group' ? ' sq' : ''}`);
  if (!one.id) {
    round.classList.add('hx');
    round.append(icon(one.type === 'group' ? 'i-users' : 'i-user'));
  } else {
    round.classList.add(`h${(Number(one.id.slice(-3)) % HUES) + 1}`);
    round.textContent = Array.from(one.name)[0] || '?';
    const picture = element('img');
    picture.alt = '';
    picture.src = `/avatar/${one.type === 'group' ? 'group' : 'user'}/${one.id}`;
    picture.addEventListener('load', () => round.replaceChildren(picture));
  }
  holder.append(round);
  if (id) {
    const badge = element('span', `pf-badge pf-${id}`);
    badge.append(mark(platform(id)));
    holder.append(badge);
  }
  return holder;
}
