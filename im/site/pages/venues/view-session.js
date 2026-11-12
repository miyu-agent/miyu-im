// 会话设置卡片左栏「实时日志」下面的「查看会话」（施工 I-232，todolist 第 59 条，项目主人 11-12 定）：装了网页软件的才有，点了
// 在新标签页打开网页软件、直接是这个群或私聊的 AI 会话（场所会话在网页那边只读：不出输入框、不能改）。哪个会话照宿主的
// sessions.list 找：同一个群、私聊的主线（并行时还有支线，看主线）；还没说过话、没有会话的说一句。
import { element, icon, say } from '../../shell/i18n.js';
import { lucide } from '../../shell/lucide.js';
import { toast } from '../../shell/dialog.js';

/** 「查看会话」那一项：`card` 是这张卡（kind 是 group、private，id 是群号、对方的号），`request` 调宿主。 */
export function viewSessionButton(card, request) {
  const button = element('button', 'd-tab d-tab-out');
  button.append(element('span', 'grow', say('venue/view-session')), lucide('external-link', icon('i-link')));
  button.addEventListener('click', async () => {
    const tab = window.open('', '_blank');
    try {
      const session = sessionOf(await request('sessions.list'), card);
      if (!session) {
        tab?.close();
        toast(say('venue/view-session-none'));
        return;
      }
      const { url } = await request('web.open', { session });
      if (tab) {
        tab.opener = null;
        tab.location.href = url;
      } else location.href = url;
    } catch {
      tab?.close();
      toast(say('failed/web'));
    }
  });
  return button;
}

/** sessions.list 的回答 `listed` 里卡片 `card` 那个群、私聊的会话编号：主线优先，没有主线标记的取最新的那个；没有的是空的。 */
export function sessionOf(listed, card) {
  const mine = (listed?.connections || [])
    .flatMap((group) => group.sessions || [])
    .filter((one) => one.kind === card.kind && String(one.id) === String(card.id));
  return (mine.find((one) => one.main) || mine[0])?.session || null;
}
