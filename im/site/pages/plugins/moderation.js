// 违规关键词（施工 I-16 照旧版设置页做在设置页里；施工 I-113，项目主人 11-02：违规判断是群管理的功能，挪到群管理插件的设置里）：群里
// 出现这些词时交给判断模型看一眼严重程度，门槛在群卡「高级」里。一份全局的（宿主的 moderation.get、moderation.set：存成系统那一份，
// 整份替换出厂的），能回到出厂的。插件详情「设置」那一页里，群管理（[`ADMIN`]）在它自己的配置项下面多这一段。
import { element, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { tagInput } from '../../shell/tags.js';

/** 群管理插件的编号（内置在 OneBot 驱动里）。 */
export const ADMIN = 'im-qq-admin';

/** 违规关键词那一段：先画「读取中」，读到了换上。 */
export function moderationSection(request) {
  const box = element('div', 'plug-settings moderation-words');
  const load = async () => {
    let words;
    try {
      words = await request('moderation.get');
    } catch {
      box.replaceChildren(element('p', 'faint', say('plugins-page/settings-failed')));
      return;
    }
    const put = async (keywords) => {
      try {
        const answer = await request('moderation.set', { keywords });
        if (!answer.saved) toast(say('failed/save'));
      } catch {
        toast(say('failed/save'));
        load();
      }
    };
    const head = element('div', 'group-name', say('settings-page/moderation'));
    const tags = tagInput({ items: words.keywords || [], save: (list) => put(list) });
    const rows = [head, line(say('settings-page/moderation-words'), say('settings-page/moderation-words-hint'), tags)];
    if (words.own) {
      const back = element('button', 'btn', say('settings-page/moderation-factory'));
      back.addEventListener('click', async () => {
        await put(null);
        load();
      });
      rows.push(line(say('settings-page/moderation-factory-label'), null, back));
    }
    box.replaceChildren(...rows);
  };
  box.append(element('p', 'faint', say('plugins-page/loading')));
  load();
  return box;
}

/** 一行：名字、小字、控件（同插件设置的行）。 */
function line(label, hint, control) {
  const row = element('div', 'row');
  const text = element('div', 'label', label);
  if (hint) text.append(element('small', null, hint));
  row.append(text, control);
  return row;
}
