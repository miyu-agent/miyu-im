// 「模型」页（设计 09，施工 I-149 起；照网页软件设置页的「模型」）：供应商（providers.js）；默认模型、模型池在 I-151 加。读经转发口
// model.list、config.schema、config.get，存一律 config.set（state.js）。添加供应商这一步先开手写的表（provider-form.js），I-150 换成
// 照核心目录选的那几屏。
import { element, say } from '../../shell/i18n.js';
import { modelState } from './state.js';
import { drawProviders } from './providers.js';
import { openProviderForm } from './provider-form.js';

/** 在 `view` 里起模型页：`request` 调宿主的方法。交回 update()、event()、dispose()。 */
export function mountModels(root, { request }) {
  let disposed = false;
  // 页面记着的：选中哪一家、搜的字、测试结果、正在改密钥的那一家、密钥是粘贴还是环境变量。
  const view = { provider: null, query: {}, tested: null, keyEdit: null, keyKind: 'secret' };
  const draw = () => {
    if (disposed) return;
    const head = element('div', 'view-head');
    head.append(element('h1', null, say('nav/models')));
    if (state.failed) return root.replaceChildren(head, element('p', 'faint', state.failed));
    if (!state.models) return root.replaceChildren(head, element('p', 'faint', say('models/loading')));
    const add = () =>
      openProviderForm(state, null, (id) => {
        view.provider = id;
        draw();
      });
    root.replaceChildren(head, drawProviders(state, view, draw, add));
  };
  const state = modelState(request, () => {
    draw();
    state.onDetail?.();
  });
  draw();
  state.load();
  return {
    update() {},
    event() {},
    dispose() {
      disposed = true;
    },
  };
}
