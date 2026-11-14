// 日期、时刻（施工 I-242，2026-11-14 项目主人整体过界面，miyu-ui 转来：消息记录「从/到」用浏览器自带的日期框，和网页软件不一致）：照网页软件
// 定时任务包的 dateField、clockField（miyu-web web-demo/packages/schedules/pickers.js，蓝图 web.md「定时任务」第 1 条）的样子画——
// 日期是一个像下拉的按钮，点了开月历浮层（顶上「今天」「明天」、右边翻月、周一开头、今天描一圈、选中的主色底）；时刻是能直接打字的框，
// 点了开「时」「分」两列。浮层挂在 body 上、照锚点摆（下面放不下往上开），点外面、Esc 收起，不收浮窗。
//
// 交回的元素照输入框用：`value` 读写（日期 `YYYY-MM-DD`、时刻 `HH:MM`，空的是空字），选了、改了发 `change`，调的一方不用改。
import { element, icon, say } from './i18n.js';

/** 分钟一格几分钟（同网页软件）。 */
const MINUTE_STEP = 5;

/** 本地日期写成 `YYYY-MM-DD`。 */
export function ymd(d) {
  const two = (x) => String(x).padStart(2, '0');
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}`;
}

/** 月历的格子：`year` 年 `month` 月（0 起）那一页，周一开头、整周排满（前后补别的月的日子）。纯的，同网页软件 model.js 的 monthCells。 */
export function monthCells(year, month, now) {
  const first = new Date(year, month, 1);
  const lead = (first.getDay() + 6) % 7;
  const today = ymd(now);
  const last = new Date(year, month + 1, 0).getDate();
  const count = Math.ceil((lead + last) / 7) * 7;
  return Array.from({ length: count }, (_, i) => {
    const d = new Date(year, month, 1 - lead + i);
    const date = ymd(d);
    return { date, day: d.getDate(), inMonth: d.getMonth() === month, past: date < today, future: date > today, today: date === today };
  });
}

/** 认时刻：`8`、`8:5`、`0805`、`08：05` 都认成 `HH:MM`；认不得的是空的。纯的，同网页软件 model.js 的 parseClock。 */
export function parseClock(text) {
  const s = String(text ?? '').trim().replace(/[：.]/g, ':');
  let m = /^(\d{1,2})(?::(\d{1,2}))?$/.exec(s);
  if (!m) {
    const d = /^(\d{3,4})$/.exec(s);
    if (!d) return null;
    m = [s, d[1].slice(0, -2), d[1].slice(-2)];
  }
  const hh = Number(m[1]);
  const mm = m[2] === undefined ? 0 : Number(m[2]);
  if (!(hh >= 0 && hh <= 23 && mm >= 0 && mm <= 59)) return null;
  return `${String(hh).padStart(2, '0')}:${String(mm).padStart(2, '0')}`;
}

/** 日期按钮上的字：今天、明天写「今天」「明天」，别的「10月28日 周二」；空的写 `empty`。纯的（字照语言）。 */
export function dateLabel(date, now, empty) {
  if (!date) return empty;
  const today = ymd(now);
  const tomorrow = ymd(new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1));
  if (date === today) return say('pick/today');
  if (date === tomorrow) return say('pick/tomorrow');
  const [y, m, d] = date.split('-').map(Number);
  const shown = new Date(y, m - 1, d);
  const key = y === now.getFullYear() ? 'pick/date' : 'pick/date-year';
  return say(key, { year: y, month: m, day: d, weekday: say(`pick/weekday-${shown.getDay()}`) });
}

/** 在 `anchor` 下面开一块浮层（左边对齐，下面放不下往上开，右边出界往左挪）；点外面、Esc 收起。交回收起它的函数。 */
function float(anchor, pop) {
  document.querySelector('.dp-float')?.dispatchEvent(new CustomEvent('dp-dismiss'));
  pop.classList.add('dp-float');
  document.body.append(pop);
  const at = anchor.getBoundingClientRect();
  const left = Math.max(8, Math.min(at.left, window.innerWidth - pop.offsetWidth - 8));
  const below = at.bottom + 4;
  const top = below + pop.offsetHeight > window.innerHeight - 8 ? Math.max(8, at.top - pop.offsetHeight - 4) : below;
  pop.style.left = `${left}px`;
  pop.style.top = `${top}px`;
  const outside = (event) => {
    if (!pop.contains(event.target) && !anchor.contains(event.target)) close();
  };
  // Esc 先收浮层：抓在最前面，收了就不再往下传（浮窗不跟着收）。
  const key = (event) => {
    if (event.key !== 'Escape') return;
    event.stopPropagation();
    event.preventDefault();
    close();
  };
  const close = () => {
    document.removeEventListener('pointerdown', outside, true);
    document.removeEventListener('keydown', key, true);
    pop.remove();
  };
  pop.addEventListener('dp-dismiss', close);
  document.addEventListener('pointerdown', outside, true);
  document.addEventListener('keydown', key, true);
  return close;
}

/**
 * 日期：一个像下拉的按钮，点了开月历。`options`：
 * - `empty`：空着是什么意思（「不限」这类），给了的能清空（浮层里多一个「清除」）；不给的一开始是今天。
 * - `after`：只能选今天和以后（定时任务的某一刻），过去的灰着点不了。`before`：只能选今天和以前（清理记录），以后的灰着。
 * 交回按钮：`value` 读写 `YYYY-MM-DD`，选了发 `change`。
 */
export function dateField(options = {}) {
  let now = options.empty ? '' : ymd(new Date());
  const button = element('button', 'pk-trigger dp-date');
  button.type = 'button';
  const label = element('span', 'pk-label');
  button.append(label, icon('i-down'));
  const paint = () => {
    label.textContent = dateLabel(now, new Date(), options.empty || '');
    button.classList.toggle('dp-empty', !now);
  };
  const set = (next) => {
    now = next;
    paint();
    button.dispatchEvent(new Event('change'));
  };
  Object.defineProperty(button, 'value', {
    get: () => now,
    set: (next) => {
      now = /^\d{4}-\d{2}-\d{2}$/.test(next || '') ? next : options.empty ? '' : ymd(new Date());
      paint();
    },
  });
  paint();
  button.addEventListener('click', () => {
    if (document.querySelector('.dp-float.dp-cal')) return;
    const today = new Date();
    const start = now ? now.split('-').map(Number) : [today.getFullYear(), today.getMonth() + 1];
    let view = { y: start[0], m: start[1] - 1 };
    const title = element('strong', 'dp-cal-title');
    const grid = element('div', 'dp-cal-grid');
    let shut = () => {};
    const off = (cell) => (options.after && cell.past) || (options.before && cell.future);
    const draw = () => {
      title.textContent = say('pick/month', { year: view.y, month: view.m + 1 });
      const heads = [1, 2, 3, 4, 5, 6, 0].map((d) => element('span', 'dp-cal-wd', say(`pick/wd-${d}`)));
      const days = monthCells(view.y, view.m, today).map((cell) => {
        const names = ['dp-cal-day', cell.inMonth ? '' : 'other', cell.today ? 'today' : '', cell.date === now ? 'on' : ''];
        const day = element('button', names.filter(Boolean).join(' '), String(cell.day));
        day.type = 'button';
        day.disabled = off(cell);
        day.addEventListener('click', () => {
          set(cell.date);
          shut();
        });
        return day;
      });
      grid.replaceChildren(...heads, ...days);
    };
    const turn = (by) => {
      const d = new Date(view.y, view.m + by, 1);
      view = { y: d.getFullYear(), m: d.getMonth() };
      draw();
    };
    const quick = (key, days) => {
      const quickButton = element('button', 'btn small', say(key));
      quickButton.type = 'button';
      const date = ymd(new Date(today.getFullYear(), today.getMonth(), today.getDate() + days));
      quickButton.disabled = (options.after && days < 0) || (options.before && days > 0);
      quickButton.addEventListener('click', () => {
        set(date);
        shut();
      });
      return quickButton;
    };
    const arrow = (id, key, by) => {
      const turnButton = element('button', 'icon-btn dp-cal-turn');
      turnButton.type = 'button';
      turnButton.title = say(key);
      turnButton.setAttribute('aria-label', say(key));
      turnButton.append(icon(id));
      turnButton.addEventListener('click', () => turn(by));
      return turnButton;
    };
    const head = element('div', 'dp-cal-head');
    head.append(quick('pick/today', 0), options.before ? quick('pick/yesterday', -1) : quick('pick/tomorrow', 1));
    if (options.empty) {
      const clear = element('button', 'btn small ghost', say('pick/clear'));
      clear.type = 'button';
      clear.addEventListener('click', () => {
        set('');
        shut();
      });
      head.append(clear);
    }
    head.append(element('span', 'grow'), arrow('i-left', 'pick/prev', -1), title, arrow('i-right', 'pick/next', 1));
    const pop = element('div', 'dp-pop dp-cal');
    pop.append(head, grid);
    draw();
    shut = float(button, pop);
  });
  return button;
}

/**
 * 时刻：一格能打字的框（失焦、回车时认字，认不得的回到原来的），点了开「时」「分」两列；点「时」留着、点「分」收起。交回输入框：
 * `value` 照原样读写 `HH:MM`，选了、认了字发 `change`。
 */
export function clockField(value = '') {
  const input = element('input', 'input dp-clock');
  input.type = 'text';
  input.inputMode = 'numeric';
  input.autocomplete = 'off';
  input.spellcheck = false;
  input.placeholder = '08:00';
  input.value = parseClock(value) ?? '';
  let last = input.value;
  const set = (next) => {
    input.value = next;
    last = next;
    input.dispatchEvent(new Event('change'));
  };
  const commit = () => {
    if (input.value.trim() === '') return set('');
    const got = parseClock(input.value);
    if (got) {
      if (got !== last || input.value !== got) set(got);
    } else input.value = last;
  };
  input.addEventListener('blur', commit);
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      commit();
    }
  });
  let shut = null;
  const open = () => {
    if (shut && document.querySelector('.dp-float.dp-clock-pop')) return;
    const now = parseClock(input.value) ?? '08:00';
    const [hh, mm] = now.split(':').map(Number);
    const two = (x) => String(x).padStart(2, '0');
    const column = (values, on, pick, key) => {
      const list = element('div', 'dp-clock-col');
      list.setAttribute('role', 'listbox');
      list.setAttribute('aria-label', say(key));
      for (const v of values) {
        const item = element('button', v === on ? 'dp-clock-item on' : 'dp-clock-item', two(v));
        item.type = 'button';
        item.addEventListener('click', () => pick(v, list));
        list.append(item);
      }
      return list;
    };
    const mark = (list, v) => {
      for (const item of list.children) item.classList.toggle('on', item.textContent === two(v));
    };
    const current = () => (parseClock(input.value) ?? now).split(':');
    const hours = column([...Array(24).keys()], hh, (v, list) => {
      set(`${two(v)}:${current()[1]}`);
      mark(list, v);
    }, 'pick/hour');
    const nearest = (Math.round(mm / MINUTE_STEP) * MINUTE_STEP) % 60;
    const minutes = column(Array.from({ length: 60 / MINUTE_STEP }, (_, i) => i * MINUTE_STEP), nearest, (v) => {
      set(`${current()[0]}:${two(v)}`);
      shut?.();
    }, 'pick/minute');
    const head = element('div', 'dp-clock-head');
    head.append(element('span', null, say('pick/hour')), element('span', null, say('pick/minute')));
    const cols = element('div', 'dp-clock-cols');
    cols.append(hours, minutes);
    const pop = element('div', 'dp-pop dp-clock-pop');
    pop.append(head, cols);
    shut = float(input, pop);
    for (const col of [hours, minutes]) col.querySelector('.on')?.scrollIntoView({ block: 'center' });
  };
  input.addEventListener('focus', open);
  input.addEventListener('click', open);
  return input;
}
