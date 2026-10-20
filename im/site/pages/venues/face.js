// 会话的头像（施工 I-7 上，I-7 上修去掉平台角标：会话页是按平台分的）：群是圆角方块，人是圆的。先画名字的第一个字（照号挑一种底色），
// 页面服务转来的头像（/avatar/group/<群号>、/avatar/user/<号>）到了再换上（shell/avatar.js：取到过的直接画）；兜底卡画图标。
import { element, icon } from '../../shell/i18n.js';
import { picture } from '../../shell/avatar.js';

const HUES = 5;

/** 一张卡（model.js）的头像：会话默认值那张画齿轮。 */
export function faceOf(one) {
  if (one.kind !== 'defaults') return face(one.type, one.id, one.name);
  const round = element('span', 'avatar sq hx');
  round.append(icon('i-gear'));
  return round;
}

/** `type`（group、private）的号是 `id`、名字是 `name` 的头像；没有号的是兜底卡，画图标。 */
export function face(type, id, name) {
  const round = element('span', `avatar${type === 'group' ? ' sq' : ''}`);
  if (!id) {
    round.classList.add('hx');
    round.append(icon(type === 'group' ? 'i-users' : 'i-user'));
    return round;
  }
  round.classList.add(`h${(Number(String(id).slice(-3)) % HUES) + 1}`);
  round.textContent = Array.from(name || '?')[0];
  picture(round, `/avatar/${type === 'group' ? 'group' : 'user'}/${id}`);
  return round;
}
