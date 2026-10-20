// 会话浮窗里写时长、时段的格（从 controls.js 挪出来）：限流（几条 / 几秒）、睡眠（几点到几点，能选「不睡」）。改了调 `save(value)`，
// null 是恢复默认。
import { element, say } from '../../shell/i18n.js';

/** 限流：几条 / 几秒（规则写 `N/Ms`，`0` 是不限）。两格都空是恢复默认。 */
export function rate(mine, fallback, save) {
  const box = element('span', 'inline');
  const turns = element('input', 'input num');
  const span = element('input', 'input num');
  const [fallTurns, fallSpan] = parseRate(fallback);
  const [myTurns, mySpan] = parseRate(mine);
  turns.value = mine === undefined ? '' : myTurns;
  span.value = mine === undefined ? '' : mySpan;
  turns.placeholder = fallTurns;
  span.placeholder = fallSpan;
  const changed = () => {
    const [count, length] = [turns.value.trim(), span.value.trim()];
    if (count === '' && length === '') return save(null);
    if (count === '0') return save('0');
    return save(`${count || fallTurns}/${length || fallSpan}s`);
  };
  turns.addEventListener('change', changed);
  span.addEventListener('change', changed);
  box.append(turns, document.createTextNode(say('unit/turns-per')), span, document.createTextNode(say('unit/seconds')));
  return box;
}

/** 睡眠：几点到几点（规则写 `HH:MM-HH:MM`），后面「不睡」点亮了存 `off`、两格藏起来；再点一下灭了，两格填着的存时段、空的恢复默认。
 *  两格都空是恢复默认。没写的照继承的写一行淡字。 */
export function sleep(mine, fallback, save) {
  const box = element('span', 'inline sleep');
  const times = element('span', 'inline');
  const from = element('input', 'input time');
  const to = element('input', 'input time');
  from.type = 'time';
  to.type = 'time';
  const [myFrom, myTo] = typeof mine === 'string' && mine.includes('-') ? mine.split('-') : ['', ''];
  const [fallFrom, fallTo] = typeof fallback === 'string' && fallback.includes('-') ? fallback.split('-') : ['', ''];
  from.value = myFrom || '';
  to.value = myTo || '';
  from.title = fallFrom;
  to.title = fallTo;
  const off = element('button', mine === 'off' ? 'pill on' : 'pill', say('field/sleep-off'));
  off.type = 'button';
  times.hidden = mine === 'off';
  const span = () => (from.value || to.value ? `${from.value || fallFrom}-${to.value || fallTo}` : null);
  const changed = () => save(span());
  from.addEventListener('change', changed);
  to.addEventListener('change', changed);
  off.addEventListener('click', () => {
    const next = !off.classList.contains('on');
    off.classList.toggle('on', next);
    times.hidden = next;
    save(next ? 'off' : span());
  });
  times.append(from, document.createTextNode('–'), to);
  box.append(times, off);
  if (mine === undefined && fallback) {
    box.append(element('small', 'faint', say('field/inherit', { value: fallback === 'off' ? say('field/sleep-off') : fallback })));
  }
  return box;
}

/** `5/300s` 拆成 ['5', '300']（时长换成秒）；不是这个写法的是两个空的，`0` 是 ['0', '']。 */
function parseRate(text) {
  if (typeof text !== 'string') return ['', ''];
  if (text === '0') return ['0', ''];
  const [count, length] = text.split('/');
  return [count || '', length ? String(seconds(length)) : ''];
}

/** 时长的写法（`30s`、`5m`、`1h`、`30`）换成秒；读不出的照原样。 */
export function seconds(text) {
  const found = /^(\d+)([smh]?)$/.exec(String(text));
  if (!found) return text;
  const unit = { '': 1, s: 1, m: 60, h: 3600 }[found[2]];
  return Number(found[1]) * unit;
}
