// 人格页（设计 09，施工 I-145；照网页软件设置页的「人格」）：一张卡一个人格，名字、简单描述，默认的那个名字后面标「默认」；文件
// 写错的那张写编号和错在哪（照核心 check 给人看的那一句、哪个文件、第几行）。点卡片开详情浮窗（editor.js），头上「＋ 新建」先只填
// 名字，建好了接着开详情。
//
// - 读经转发口：persona.list、config.get 的 persona.default；有写错的再调一次 check。打开这一页、存了、删了都重读。
// - 2026-10-08 项目主人（网页软件定的，这里照守）：来自哪一层、以谁为底、编号、分语言的名字都不显示。
import { element, icon, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { openPersona } from './editor.js';
import { imageUrl, onImage } from './images.js';

/** 在 `view` 里起人格页：`request` 调宿主的方法。交回 update()、event()、dispose()。 */
export function mountPersonas(view, { request }) {
  let disposed = false;
  const load = async () => {
    let personas;
    let chosen;
    try {
      [personas, chosen] = await Promise.all([
        core(request, 'persona.list').then((got) => got.personas || []),
        core(request, 'config.get', { keys: ['persona.default'] }),
      ]);
    } catch (error) {
      if (!disposed) view.replaceChildren(head(null), element('p', 'faint', refusalText(error)));
      return;
    }
    const problems = personas.some((one) => one.problem)
      ? await core(request, 'check').then((got) => got.problems || [], () => [])
      : [];
    if (disposed) return;
    const fallback = chosen?.items?.['persona.default']?.value ?? null;
    const hooks = { saved: load, removed: load };
    const cards = personas.map((one) =>
      one.problem
        ? badCard(one.persona, problemsOf(problems, one))
        : card(one, one.persona === fallback, imageUrl(request, 'avatar', one.persona, one.avatar), () =>
            openPersona(request, one.persona, nameOf(one), hooks),
          ),
    );
    const grid = element('div', 'persona-grid');
    grid.append(...cards);
    view.replaceChildren(
      head(() => create(request, hooks)),
      cards.length ? grid : element('p', 'faint', say('personas/none')),
    );
  };
  load();
  // 头像读到了一张新的：重画列表（卡片上画头像）。
  const stop = onImage(() => !disposed && load());
  return {
    update() {},
    event() {},
    dispose() {
      disposed = true;
      stop();
    },
  };
}

/** 头上：标题、一句这页管什么、「＋ 新建」（`onNew` 空的不画）。 */
function head(onNew) {
  const shown = element('div', 'view-head');
  const title = element('div');
  title.append(element('h1', null, say('nav/personas')), element('p', 'faint', say('personas/intro')));
  shown.append(title);
  if (onNew) {
    const add = element('button', 'btn primary persona-new');
    add.append(icon('i-plus'), element('span', null, say('personas/new')));
    add.addEventListener('click', onNew);
    shown.append(add);
  }
  return shown;
}

/** 写给人看的名字：没有显示名的写编号。 */
const nameOf = (one) => one.name || one.persona;

/** 一张卡：头像（`avatar` 是读到了的地址，没有的画名字的第一个字）、名字（默认的标「默认」）、简单描述。点了调 `open`。 */
function card(one, isDefault, avatar, open) {
  const box = element('button', 'persona-card');
  const name = nameOf(one);
  const face = element('span', 'persona-face', avatar ? '' : [...name][0] || '');
  if (avatar) {
    const shown = element('img');
    shown.src = avatar;
    shown.alt = '';
    face.append(shown);
  }
  const text = element('span', 'persona-card-text');
  const title = element('b', null, name);
  if (isDefault) title.append(element('span', 'chip', say('personas/default')));
  text.append(title);
  if (one.summary) text.append(element('small', null, one.summary));
  box.append(face, text);
  box.addEventListener('click', open);
  return box;
}

/** 写错的一张：编号（读不出名字），下面一处一行。 */
function badCard(id, problems) {
  const box = element('div', 'persona-card bad');
  const text = element('span', 'persona-card-text');
  text.append(element('b', null, id), ...problems.map((one) => element('small', null, one)));
  box.append(text);
  return box;
}

/** 写错的人格 `one` 人看的几句：check 里这个人格目录下的，找不到的退回列表里那句原话（纯逻辑，同网页软件的 problemsOf）。 */
export function problemsOf(problems, one) {
  const tail = `personas/${one.persona}/`;
  const found = problems
    .filter((problem) => problem.kind === 'persona' && typeof problem.file === 'string')
    .map((problem) => ({ problem, file: problem.file.replace(/\\/g, '/') }))
    .filter(({ file }) => file.startsWith(tail) || file.includes(`/${tail}`))
    .map(({ problem, file }) => {
      const short = file.slice(file.lastIndexOf(tail) + tail.length);
      return Number.isInteger(problem.line)
        ? say('personas/problem-line', { file: short, line: problem.line, message: problem.message })
        : say('personas/problem-file', { file: short, message: problem.message });
    });
  return found.length ? found : [one.problem || ''];
}

/** 「＋ 新建」：浮窗里先只填名字，建好了关掉、开这个人格的详情。 */
function create(request, hooks) {
  const dialog = openDialog(say('personas/new'));
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
    let made;
    try {
      made = await core(request, 'persona.set', { changes: [{ key: 'persona.name', value: typed }] });
    } catch (error) {
      go.disabled = false;
      note.hidden = false;
      note.textContent = refusalText(error);
      return;
    }
    dialog.close();
    hooks.saved();
    openPersona(request, made.persona, typed, hooks, true);
  };
  name.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.isComposing) submit();
  });
  go.addEventListener('click', submit);
  dialog.body.append(name, note);
  dialog.foot.append(go);
  name.focus();
}
