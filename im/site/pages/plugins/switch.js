// 插件的总开关（施工 I-9 上，设计 01 第四节第 4 条：核心的包开关，在这份登录的人的连接上做）：卡片、详情页头上各一个。打开时还有没
// 批的能力的，先问一句（列出那几样），批了一并打开；关掉的直接关。做完了重读插件页。
import { element, say } from '../../shell/i18n.js';
import { openDialog, toast } from '../../shell/dialog.js';
import { named } from '../venues/plugins.js';

/** 插件 `plugin` 的总开关：`context` 是 { request, reload }。 */
export function masterSwitch(plugin, context) {
  const button = element('button', plugin.enabled ? 'switch on' : 'switch');
  button.append(element('i'));
  button.title = say(plugin.enabled ? 'plugins-page/turn-off' : 'plugins-page/turn-on');
  button.addEventListener('click', async (event) => {
    event.stopPropagation();
    const on = !plugin.enabled;
    const unapproved = on ? plugin.unapproved || [] : [];
    if (unapproved.length > 0 && !(await approved(plugin, unapproved))) return;
    button.disabled = true;
    try {
      const params = { package: plugin.package, on };
      if (unapproved.length > 0) params.approve = unapproved;
      await context.request('plugins.enable', params);
    } catch (reason) {
      toast(say(reason === 'needs_approval' ? 'plugins-page/needs-approval' : 'failed/save'));
    }
    context.reload();
  });
  return button;
}

/** 问一句要不要批 `ids` 那几样能力：交回人点的是不是「批准并打开」。 */
function approved(plugin, ids) {
  return new Promise((resolve) => {
    let answered = false;
    const dialog = openDialog(say('plugins-page/approve-title', { name: named(plugin.name) || plugin.package }), () => {
      if (!answered) resolve(false);
    });
    dialog.body.append(element('p', 'soft', say('plugins-page/approve-text')));
    const list = element('ul', 'capabilities');
    for (const id of ids) {
      const known = (plugin.capabilities || []).find((one) => one.id === id);
      const item = element('li', null, known?.name || id);
      if (known?.summary) item.append(element('small', null, known.summary));
      list.append(item);
    }
    dialog.body.append(list);
    const cancel = element('button', 'btn ghost', say('dialog/cancel'));
    cancel.addEventListener('click', () => dialog.close());
    const yes = element('button', 'btn primary', say('plugins-page/approve'));
    yes.addEventListener('click', () => {
      answered = true;
      resolve(true);
      dialog.close();
    });
    dialog.foot.append(element('span', 'grow'), cancel, yes);
  });
}
