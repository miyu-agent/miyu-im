// 人格页（设计 09，施工 I-145；照网页软件设置页的「人格」）：一张卡一个人格，头像、名字、简单描述，默认的那个名字后面标「默认」；
// 文件写错的那张写编号和错在哪。点卡片开详情浮窗（editor.js），头上「＋ 新建」先只填名字，建好了接着开详情。列表部件和预设页共用
// （list.js）。
//
// - 读经转发口：persona.list、config.get 的 persona.default；有写错的再调一次 check。打开这一页、存了、删了都重读。
// - 2026-10-08 项目主人（网页软件定的，这里照守）：来自哪一层、以谁为底、编号、分语言的名字都不显示。
import { element } from '../../shell/i18n.js';
import { core, refusalText } from '../../shell/core.js';
import { openPersona } from './editor.js';
import { imageUrl, onImage } from './images.js';
import { badCard, face, grid, itemCard, listHead, nameFirst, problemsOf } from './list.js';

/** 在 `view` 里起人格页：`request` 调宿主的方法。交回 update()、event()、dispose()。 */
export function mountPersonas(view, { request }) {
  let disposed = false;
  const head = (onNew) => listHead('nav/personas', 'personas/intro', 'personas/new', onNew);
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
    const cards = personas.map((one) => {
      if (one.problem) return badCard(one.persona, problemsOf(problems, 'persona', one.persona, one.problem));
      const name = one.name || one.persona;
      const open = () => openPersona(request, one.persona, name, hooks);
      return itemCard(face(name, avatar(request, one)), name, one.persona === fallback, one.summary, open);
    });
    const create = () =>
      nameFirst(
        'personas/new',
        (name) => core(request, 'persona.set', { changes: [{ key: 'persona.name', value: name }] }).then((made) => made.persona),
        (id, name) => {
          load();
          openPersona(request, id, name, hooks, true);
        },
      );
    view.replaceChildren(head(create), grid(cards, 'personas/none'));
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

/** 人格 `one` 的头像：读到了的画图，没有、还没读到的是空的。 */
function avatar(request, one) {
  const url = imageUrl(request, 'avatar', one.persona, one.avatar);
  if (!url) return null;
  const shown = element('img');
  shown.src = url;
  shown.alt = '';
  return shown;
}
