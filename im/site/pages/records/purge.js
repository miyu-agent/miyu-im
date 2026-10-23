// 删聊天记录的浮窗（施工 C-1 中下，设计 04 第一节第 2 条；项目主人 10-23 定：只在 im 后台删，AI 不许删，能批量）：删哪些场所（勾几个、
// 全部）、哪段时间（不填是全部时间）、谁说的（号，不填是所有人），点两次「删除」才删。删的只是这个库：她当时看过的还在她的会话里。
import { element, icon, say } from '../../shell/i18n.js';
import { openDialog, toast } from '../../shell/dialog.js';
import { venueParts } from './shared.js';

/** 一天的毫秒数。 */
const DAY = 86400000;

/** 开浮窗：`venues` 是能勾的场所 [{venue, title, detail}]，`chosen` 是先勾上的几个；删完调 `done()`。 */
export function openPurge({ request, venues, chosen, done }) {
  const dialog = openDialog(say('records/purge-title'));
  const picked = new Set(chosen || []);
  const rows = new Map();
  const failed = element('p', 'failed');

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

  dialog.body.append(
    section('records/purge-venues', list),
    section('records/purge-span', span, 'records/purge-span-hint'),
    section('records/purge-who', person, 'records/purge-who-hint'),
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
    toast(say('records/purged', { count: deleted }));
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
