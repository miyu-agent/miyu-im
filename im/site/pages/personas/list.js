// 人格页、预设页共用的列表部件（设计 09，施工 I-147 从 personas.js 抽出来）：头上标题和「＋ 新建」、一张张卡片、文件写错的卡片
// （照核心 check 给人看的那一句、哪个文件、第几行）、新建先只填名字的浮窗。样子都用 personas.css 的 persona-* 那几个类。
import { element, icon, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { refusalText } from '../../shell/core.js';

/** 头上：标题 `title`、一句这页管什么 `intro`、「＋ 新建」（字是 `add`，`onNew` 空的不画）。都是字表的键。 */
export function listHead(title, intro, add, onNew) {
  const shown = element('div', 'view-head');
  const named = element('div');
  named.append(element('h1', null, say(title)));
  if (intro) named.append(element('p', 'faint', say(intro)));
  shown.append(named);
  if (onNew) {
    const button = element('button', 'btn primary persona-new');
    button.append(icon('i-plus'), element('span', null, say(add)));
    button.addEventListener('click', onNew);
    shown.append(button);
  }
  return shown;
}

/** 卡片网格：`cards` 空的写 `none`（字表的键）。 */
export function grid(cards, none) {
  if (!cards.length) return element('p', 'faint', say(none));
  const box = element('div', 'persona-grid');
  box.append(...cards);
  return box;
}

/** 一张卡：左边 `face`、名字 `name`（`isDefault` 的标「默认」）、简单描述 `summary`。点了调 `open`。 */
export function itemCard(face, name, isDefault, summary, open) {
  const box = element('button', 'persona-card');
  const text = element('span', 'persona-card-text');
  const title = element('b', null, name);
  if (isDefault) title.append(element('span', 'chip', say('personas/default')));
  text.append(title);
  if (summary) text.append(element('small', null, summary));
  box.append(face, text);
  box.addEventListener('click', open);
  return box;
}

/** 卡片左边的圆：`inner` 是元素（头像、图标）的放进去，没有的写 `name` 的第一个字。 */
export function face(name, inner) {
  const shown = element('span', 'persona-face', inner ? '' : [...name][0] || '');
  if (inner) shown.append(inner);
  return shown;
}

/** 写错的一张：编号（读不出名字），下面一处一行。 */
export function badCard(id, problems) {
  const box = element('div', 'persona-card bad');
  const text = element('span', 'persona-card-text');
  text.append(element('b', null, id), ...problems.map((one) => element('small', null, one)));
  box.append(text);
  return box;
}

/** 写错的人格、预设（`kind` 是 persona、preset，编号 `id`，列表里的原话 `said`）人看的几句：check 里落在它文件上的，找不到的退回原话
 *  （纯逻辑，同网页软件的 problemsOf）。 */
export function problemsOf(problems, kind, id, said) {
  const tail = kind === 'persona' ? `personas/${id}/` : `presets/${id}.toml`;
  const found = problems
    .filter((problem) => problem.kind === kind && typeof problem.file === 'string')
    .map((problem) => ({ problem, file: problem.file.replace(/\\/g, '/') }))
    .filter(({ file }) => file.startsWith(tail) || file.includes(`/${tail}`))
    .map(({ problem, file }) => {
      const short = kind === 'persona' ? file.slice(file.lastIndexOf(tail) + tail.length) : `${id}.toml`;
      return Number.isInteger(problem.line)
        ? say('personas/problem-line', { file: short, line: problem.line, message: problem.message })
        : say('personas/problem-file', { file: short, message: problem.message });
    });
  return found.length ? found : [said || ''];
}

/** 「＋ 新建」：浮窗（标题 `title`，字表的键）里先只填名字，`make(name)` 建好交回编号，关掉以后调 `made(id, name)`。 */
export function nameFirst(title, make, made) {
  const dialog = openDialog(say(title));
  const name = element('input', 'input persona-input');
  name.placeholder = say('personas/name');
  const note = element('p', 'persona-note');
  note.hidden = true;
  const go = element('button', 'btn primary', say('personas/create'));
  go.disabled = true;
  name.addEventListener('input', () => {
    go.disabled = !name.value.trim();
  });
  const submit = async () => {
    const typed = name.value.trim();
    if (!typed) return;
    go.disabled = true;
    let id;
    try {
      id = await make(typed);
    } catch (error) {
      go.disabled = false;
      note.hidden = false;
      note.textContent = refusalText(error);
      return;
    }
    dialog.close();
    made(id, typed);
  };
  name.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.isComposing) submit();
  });
  go.addEventListener('click', submit);
  dialog.body.append(name, note);
  dialog.foot.append(element('span', 'grow'), go);
  name.focus();
}
