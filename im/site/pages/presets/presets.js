// 预设页（设计 09，施工 I-147；照网页软件设置页的「预设」）：一张卡一个预设，图标、名字、简单描述，默认的那个标「默认」（没设默认的
// 照出厂的 full）；文件写错的那张写编号和错在哪。点卡片开详情浮窗（editor.js），「＋ 新建」先只填名字，建好了开详情（新建的功能
// 默认全开，2026-10-08）。列表部件和人格页共用（personas/list.js）。
//
// - 读经转发口：preset.list、config.get 的 preset.default；有写错的再调一次 check。打开这一页、存了、删了都重读。
// - preset.list 末尾 preset 是 null 的那一项（旧核心的「无工具」）不是能编辑的预设，不出卡片。
import { element } from '../../shell/i18n.js';
import { core, refusalText } from '../../shell/core.js';
import { lucide } from '../../shell/lucide.js';
import { badCard, face, grid, itemCard, listHead, nameFirst, problemsOf } from '../personas/list.js';
import { openPreset } from './editor.js';

/** 在 `view` 里起预设页：`request` 调宿主的方法。交回 update()、event()、dispose()。 */
export function mountPresets(view, { request }) {
  let disposed = false;
  const head = (onNew) => listHead('nav/presets', 'presets/intro', 'presets/new', onNew);
  const load = async () => {
    let presets;
    let chosen;
    try {
      [presets, chosen] = await Promise.all([
        core(request, 'preset.list').then((got) => (got.presets || []).filter((one) => one.preset != null)),
        core(request, 'config.get', { keys: ['preset.default'] }),
      ]);
    } catch (error) {
      if (!disposed) view.replaceChildren(head(null), element('p', 'faint', refusalText(error)));
      return;
    }
    const problems = presets.some((one) => one.problem)
      ? await core(request, 'check').then((got) => got.problems || [], () => [])
      : [];
    if (disposed) return;
    const fallback = chosen?.items?.['preset.default']?.value ?? 'full';
    const hooks = { saved: load, removed: load };
    const cards = presets.map((one) => {
      if (one.problem) return badCard(one.preset, problemsOf(problems, 'preset', one.preset, one.problem));
      const name = one.name || one.preset;
      const shown = one.icon ? lucide(one.icon, element('span', null, [...name][0] ?? '')) : null;
      return itemCard(face(name, shown), name, one.preset === fallback, one.summary, () =>
        openPreset(request, one.preset, name, hooks),
      );
    });
    const create = () =>
      nameFirst(
        'presets/new',
        (name) => core(request, 'preset.set', { changes: [{ key: 'preset.name', value: name }] }).then((made) => made.preset),
        (id, name) => {
          load();
          openPreset(request, id, name, hooks);
        },
      );
    view.replaceChildren(head(create), grid(cards, 'presets/none'));
  };
  load();
  return {
    update() {},
    event() {},
    dispose() {
      disposed = true;
    },
  };
}
