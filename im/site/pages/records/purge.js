// 清理聊天记录的浮窗（施工 C-1 中下，设计 04 第一节第 2 条；项目主人 10-23 定：只在 im 后台删，AI 不许删，能批量；I-230 起叫「清理」、
// 带几个快捷的范围，todolist 第 53 条）：清哪些场所（勾几个、全部）、范围（保留近 7、30、90 天，全部清空，自定义：哪段时间、谁说的），
// 点两次「清理」才清。清完整理一遍（records.compact），把空出来的还给硬盘，说空出了多少。清的只是这个库：她当时看过的还在她的会话里。
import { element, icon, say } from '../../shell/i18n.js';
import { openDialog, toast } from '../../shell/dialog.js';
import { sizeText, venueParts } from './shared.js';

/** 一天的毫秒数。 */
const DAY = 86400000;

/** 快捷的范围：保留近几天（0 是全部清空），`null` 是自定义；默认保留近 30 天（施工 I-230，项目主人 11-12 定）。 */
const KEEPS = [[7, 'records/keep-7'], [30, 'records/keep-30'], [90, 'records/keep-90'], [0, 'records/keep-none'], [null, 'records/keep-custom']];

/** 开浮窗：`venues` 是能勾的场所 [{venue, title, detail}]，`chosen` 是先勾上的几个；删完调 `done()`。 */
export function openPurge({ request, venues, chosen, done }) {
  const dialog = openDialog(say('records/purge-title'));
  const picked = new Set(chosen || []);
  const rows = new Map();
  const failed = element('p', 'failed');
  let keep = 30;

  // 现在占多大：自己问一次（场所页里开的也有）。
  const now = element('p', 'muted records-purge-now');
  request('records.venues').then((listed) => {
    if (listed?.bytes != null) now.textContent = say('records/purge-now', { size: sizeText(listed.bytes) });
  }).catch(() => {});

  // 场所：不止一个的，最上面一行「全部场所」。
  const list = element('div', 'records-checks');
  const all = venues.length > 1 ? checkRow(say('records/purge-all'), null) : null;
  if (all) {
    all.row.addEventListener('click', () => {
      const on = picked.size < venues.length;
      for (const one of venues) on ? picked.add(one.venue) : picked.delete(one.venue);
      paint();
    });
    list.append(all.row);
  }
  for (const one of venues) {
    const row = checkRow(one.title, one.detail);
    row.row.addEventListener('click', () => {
      picked.has(one.venue) ? picked.delete(one.venue) : picked.add(one.venue);
      paint();
    });
    rows.set(one.venue, row);
    list.append(row.row);
  }

  // 时间、人。
  const from = dateInput();
  const to = dateInput();
  const span = element('div', 'records-purge-line');
  span.append(element('span', 'muted', say('records/filter-from')), from, element('span', 'muted', say('records/filter-to')), to);
  const person = element('input', 'input records-purge-person');
  person.placeholder = say('records/purge-person');
  person.inputMode = 'numeric';

  // 范围：几个快捷的，选「自定义」才露出时间、人两段。
  const seg = element('div', 'seg records-keep');
  const keepHint = element('div', 'muted records-purge-hint');
  const custom = [section('records/purge-span', span, 'records/purge-span-hint'), section('records/purge-who', person, 'records/purge-who-hint')];
  const keepButtons = KEEPS.map(([days, key]) => {
    const button = element('button', null, say(key));
    button.addEventListener('click', () => {
      keep = days;
      paintKeep();
      disarm();
    });
    seg.append(button);
    return [days, button];
  });
  const keepBox = section('records/purge-keep', seg);
  keepBox.append(keepHint);
  const paintKeep = () => {
    for (const [days, button] of keepButtons) button.classList.toggle('on', days === keep);
    for (const one of custom) one.hidden = keep !== null;
    if (keep === null) keepHint.textContent = '';
    else if (keep === 0) keepHint.textContent = say('records/keep-none-hint');
    else keepHint.textContent = say('records/keep-before', { date: new Date(Date.now() - keep * DAY).toLocaleDateString() });
  };

  dialog.body.append(
    now,
    section('records/purge-venues', list),
    keepBox,
    ...custom,
    element('p', 'muted records-purge-note', say('records/purge-note')),
    failed,
  );

  const cancel = element('button', 'btn ghost', say('records/cancel'));
  cancel.addEventListener('click', () => dialog.close());
  const go = element('button', 'btn danger', say('records/delete'));
  let armed = false;
  const disarm = () => {
    armed = false;
    go.textContent = say('records/delete');
  };
  for (const one of [from, to, person]) one.addEventListener('input', disarm);
  go.addEventListener('click', async () => {
    const scope = scopeOf();
    if (!scope) return;
    if (!armed) {
      armed = true;
      go.textContent = say('records/delete-again');
      return;
    }
    go.disabled = true;
    let deleted = 0;
    try {
      for (const params of scope) deleted += (await request('records.delete', params)).deleted || 0;
    } catch {
      go.disabled = false;
      disarm();
      failed.textContent = say('records/delete-failed');
      return;
    }
    // 整理一遍把空出来的还给硬盘；整理没成的照样算清理了，只是不说空出多少。
    let freed = null;
    try {
      const compacted = await request('records.compact');
      freed = Math.max(0, (compacted.before || 0) - (compacted.after || 0));
    } catch {
      freed = null;
    }
    toast(freed == null ? say('records/purged', { count: deleted }) : say('records/purged-freed', { count: deleted, size: sizeText(freed) }));
    dialog.close();
    done();
  });
  dialog.foot.append(element('span', 'grow'), cancel, go);

  /** 照勾的、写的拼要发的几次 records.delete；写错了的写在底下、交回 null。 */
  const scopeOf = () => {
    failed.textContent = '';
    const whole = venues.length > 0 && picked.size === venues.length && venues.length > 1;
    const base = {};
    if (!whole) base.venues = [...picked];
    if (keep === 0) return [whole ? { all: true } : base];
    if (keep !== null) return [{ ...base, to: Date.now() - keep * DAY }];
    if (from.value) base.from = new Date(`${from.value}T00:00`).getTime();
    if (to.value) base.to = new Date(`${to.value}T00:00`).getTime() + DAY;
    if (base.from && base.to && base.from >= base.to) {
      failed.textContent = say('records/purge-bad-span');
      return null;
    }
    const number = person.value.trim();
    if (number && !/^\d+$/.test(number)) {
      failed.textContent = say('records/purge-bad-person');
      return null;
    }
    if (!number) return [whole && !base.from && !base.to ? { all: true } : base];
    // 人照平台身份删：勾的场所在哪几个平台，每个平台删一次。
    const platforms = new Set((whole ? venues : venues.filter((one) => picked.has(one.venue))).map((one) => venueParts(one.venue).platform));
    return [...platforms].map((platform) => ({ ...base, sender: `${platform}:${number}` }));
  };

  const paint = () => {
    paintKeep();
    for (const [venue, row] of rows) row.set(picked.has(venue));
    all?.set(picked.size === venues.length);
    go.disabled = picked.size === 0;
    disarm();
  };
  paint();
}

/** 能勾的一行：名字 `title`、底下一行 `detail`。交回 { row, set(on) }。 */
function checkRow(title, detail) {
  const row = element('button', 'records-check-row');
  const box = element('span', 'records-check');
  box.append(icon('i-check'));
  const text = element('span', 'grow');
  text.append(element('span', 'name', title));
  if (detail) text.append(element('span', 'muted', detail));
  row.append(box, text);
  return { row, set: (on) => row.classList.toggle('on', on) };
}

/** 一段：标题 `key`、内容 `body`、底下一行提示 `hint`。 */
function section(key, body, hint) {
  const box = element('div', 'records-purge-section');
  box.append(element('div', 'records-purge-label', say(key)), body);
  if (hint) box.append(element('div', 'muted records-purge-hint', say(hint)));
  return box;
}

/** 选日子的一格。 */
function dateInput() {
  const input = element('input', 'input');
  input.type = 'date';
  return input;
}
