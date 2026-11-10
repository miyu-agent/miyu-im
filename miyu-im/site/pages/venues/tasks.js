// 会话浮窗的「定时任务」页（设计 12 第一节第 3 条，施工 I-198）：给这个会话建定时、循环的任务，到点 AI 在这个会话里做。
//
// - 读写经宿主的 tasks.list、tasks.save、tasks.remove（卡的写法同 cards.save_one）：只有这张卡的任务，别的定时不在里面。
// - 上面一张「新建任务」：要做什么（多行字）、什么时候（每天几点、每隔多少分钟、某一刻一次、高级里直接写 cron），时区照浏览器的。
//   点了列表里的「编辑」，这一张换成改那一个，存了、取消了换回来。
// - 下面一张列表：一行一个任务：要做什么、什么时候、下一次几点、上一次的结果；能停、恢复、编辑、删（删要点两次）。
// - 时间写得不对的，核心拒的那一句照原样提示。
import { element, say } from '../../shell/i18n.js';
import { sect } from '../../shell/sect.js';
import { toast } from '../../shell/dialog.js';

/** 一个日期的「月-日 时:分」；今天的只写时刻。 */
function clock(text) {
  const at = new Date(text);
  if (Number.isNaN(at.getTime())) return '';
  const time = at.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  return at.toDateString() === new Date().toDateString() ? time : `${at.toLocaleDateString([], { month: '2-digit', day: '2-digit' })} ${time}`;
}

/** 核心的 `when` 写成一句：每天几点、每隔多久、哪一刻一次；别的 cron 照原样写。 */
export function whenText(when) {
  if (when?.at) return say('tasks/when-once', { at: clock(when.at) });
  if (when?.every) {
    const minutes = Math.round(when.every / 60);
    return minutes % 60 === 0 ? say('tasks/when-hours', { count: minutes / 60 }) : say('tasks/when-minutes', { count: minutes });
  }
  const daily = /^(\d{1,2}) (\d{1,2}) \* \* \*$/.exec(when?.cron || '');
  if (daily) return say('tasks/when-daily', { at: `${daily[2].padStart(2, '0')}:${daily[1].padStart(2, '0')}` });
  return when?.cron || '';
}

/** 表单里填的写成核心的 `when`；填得不全的是空的。 */
export function whenOf(kind, value, zone) {
  if (kind === 'daily') {
    const picked = /^(\d{1,2}):(\d{2})$/.exec(value || '');
    return picked ? { cron: `${Number(picked[2])} ${Number(picked[1])} * * *`, recurring: true, zone } : null;
  }
  if (kind === 'every') {
    const minutes = Number(value);
    return Number.isInteger(minutes) && minutes >= 1 ? { every: minutes * 60 } : null;
  }
  if (kind === 'once') {
    const at = new Date(value || '');
    return Number.isNaN(at.getTime()) ? null : { at: at.toISOString() };
  }
  return (value || '').trim() ? { cron: value.trim(), recurring: true, zone } : null;
}

/** 核心的 `when` 换回表单里的（种类、填的）：编辑时用。 */
function formOf(when) {
  if (when?.at) {
    const at = new Date(when.at);
    const local = new Date(at.getTime() - at.getTimezoneOffset() * 60000).toISOString().slice(0, 16);
    return ['once', local];
  }
  if (when?.every) return ['every', String(Math.round(when.every / 60))];
  const daily = /^(\d{1,2}) (\d{1,2}) \* \* \*$/.exec(when?.cron || '');
  if (daily) return ['daily', `${daily[2].padStart(2, '0')}:${daily[1].padStart(2, '0')}`];
  return ['cron', when?.cron || ''];
}

/** 这一页：`card` 交回这张卡的写法，`request` 调宿主。交回 { box, refresh() }。 */
export function taskPage(card, request) {
  const box = element('div', 'tasks');
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  let tasks = [];
  let editing = null;
  const form = sect(say('tasks/new'));
  const list = sect(say('tasks/list'));
  box.append(form.box, list.box);

  const text = element('textarea', 'area');
  text.placeholder = say('tasks/text-hint');
  const kind = element('select', 'select');
  for (const one of ['daily', 'every', 'once', 'cron']) {
    const option = element('option', null, say(`tasks/kind-${one}`));
    option.value = one;
    kind.append(option);
  }
  const value = element('input', 'input');
  const unit = element('span', 'faint');
  const shape = () => {
    value.type = { daily: 'time', once: 'datetime-local' }[kind.value] || 'text';
    value.inputMode = kind.value === 'every' ? 'numeric' : '';
    value.placeholder = kind.value === 'cron' ? '0 8 * * 1-5' : '';
    unit.textContent = kind.value === 'every' ? say('unit/minutes') : '';
  };
  kind.addEventListener('change', () => {
    value.value = '';
    shape();
  });
  const save = element('button', 'btn primary');
  const cancel = element('button', 'btn ghost', say('dialog/cancel'));
  const fill = (task) => {
    editing = task ? task.id : null;
    form.box.querySelector('.sect-name').textContent = say(task ? 'tasks/edit' : 'tasks/new');
    save.textContent = say(task ? 'tasks/save' : 'tasks/add');
    cancel.hidden = !task;
    text.value = task ? task.text : '';
    const [shownKind, shownValue] = task ? formOf(task.when) : ['daily', ''];
    kind.value = shownKind;
    shape();
    value.value = shownValue;
  };
  cancel.addEventListener('click', () => fill(null));
  save.addEventListener('click', async () => {
    const when = whenOf(kind.value, value.value, zone);
    if (!text.value.trim() || !when) {
      toast(say('tasks/incomplete'));
      return;
    }
    save.disabled = true;
    const params = { card: card(), text: text.value.trim(), when };
    if (editing) params.id = editing;
    try {
      const answer = await request('tasks.save', params);
      if (answer.saved) {
        fill(null);
        await refresh();
      } else toast(answer.message || say('failed/save'));
    } catch {
      toast(say('failed/save'));
    }
    save.disabled = false;
  });
  const what = element('div', 'row');
  const whatLabel = element('div', 'label');
  whatLabel.append(element('div', 'label-name', say('tasks/text')));
  what.append(whatLabel, text);
  const whenRow = element('div', 'row');
  const whenLabel = element('div', 'label');
  whenLabel.append(element('div', 'label-name', say('tasks/when')), element('small', null, say('tasks/when-hint', { zone })));
  const picked = element('span', 'inline');
  picked.append(kind, value, unit);
  whenRow.append(whenLabel, picked);
  const foot = element('div', 'tasks-foot');
  foot.append(cancel, save);
  form.body.append(what, whenRow, foot);

  const act = async (params, method = 'tasks.save') => {
    try {
      const answer = await request(method, { card: card(), ...params });
      if (answer.saved === false) toast(answer.message || say('failed/save'));
    } catch {
      toast(say('failed/save'));
    }
    await refresh();
  };
  const row = (task) => {
    const line = element('div', task.paused ? 'task off' : 'task');
    const main = element('div', 'task-main');
    const notes = [whenText(task.when)];
    if (task.paused) notes.push(say('tasks/paused'));
    else if (task.next) notes.push(say('tasks/next', { at: clock(task.next) }));
    if (task.last?.at) notes.push(say(task.last.result?.startsWith('skipped') ? 'tasks/last-skipped' : 'tasks/last', { at: clock(task.last.at) }));
    main.append(element('div', 'task-text', task.text), element('small', null, notes.join(' · ')));
    const buttons = element('div', 'task-buttons');
    const pause = element('button', 'btn ghost', say(task.paused ? 'tasks/resume' : 'tasks/pause'));
    pause.addEventListener('click', () => act({ id: task.id, paused: !task.paused }));
    const edit = element('button', 'btn ghost', say('tasks/edit-button'));
    edit.addEventListener('click', () => {
      fill(task);
      text.focus();
    });
    const remove = element('button', 'btn ghost danger', say('tasks/remove'));
    let armed = false;
    remove.addEventListener('click', () => {
      if (!armed) {
        armed = true;
        remove.textContent = say('tasks/remove-again');
        return;
      }
      if (editing === task.id) fill(null);
      act({ id: task.id }, 'tasks.remove');
    });
    buttons.append(pause, edit, remove);
    line.append(main, buttons);
    return line;
  };
  const refresh = async () => {
    try {
      tasks = (await request('tasks.list', { card: card() })).tasks || [];
    } catch {
      // 问不到的：照旧画上一次的。
    }
    if (tasks.length === 0) list.body.replaceChildren(element('p', 'faint', say('tasks/none')));
    else list.body.replaceChildren(...tasks.map(row));
  };
  fill(null);
  refresh();
  return { box, refresh };
}
