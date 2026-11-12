// 新建会话（施工 I-7 上，设计 01 第八节第 3 条；I-7 上修照平台）：在这个平台上新建，有哪几种会话、号怎么叫、怎么找由平台给
// （platforms.js 的 venues）：只有一种的不用先选。先选群聊还是私聊，再一个搜索框：搜 AI 在的群、联系人，搜到了就用（带头像）；搜不到、
// 填的是号码的照号直接新建（QQ 的私聊白名单要先加，对方加 AI 好友时才自动通过，所以不能只从好友里选）；不是号码的写「没搜到」。已经
// 在名单里的不列。「添加并设置」交 cards.add（私聊的写 p:<号>），加好了关掉、交回号，会话页接着打开它的浮窗。
import { element, icon, say, spaced } from '../../shell/i18n.js';
import { openDialog, toast } from '../../shell/dialog.js';
import { venueKinds } from '../home/platforms.js';
import { face } from './face.js';

/** 号码的样子。 */
const NUMBER = /^\d{1,20}$/;

/** 最多列几个。 */
const SHOWN = 50;

/** 开新建；`context` 是 { request, platform, answer（cards.get 的回答）, listed（已经在名单里的 group:<号>、private:<号>） }，加好了
 *  调 `onAdded(type, id)`。 */
export function openCreate(context, onAdded) {
  const kinds = venueKinds(context.platform);
  const types = Object.keys(kinds);
  const dialog = openDialog(say('create/title'));
  const back = element('button', 'icon-button');
  back.title = say('wizard/back');
  back.append(icon('i-left'));
  back.addEventListener('click', () => pick());
  dialog.head.prepend(back);

  const pick = () => {
    back.hidden = true;
    dialog.setTitle(say('create/title'));
    dialog.foot.hidden = true;
    const grid = element('div', 'kinds');
    for (const type of types) {
      const button = element('button', 'kind');
      const mark = element('span', 'ico');
      mark.append(icon(type === 'group' ? 'i-users' : 'i-user'));
      const hint = spaced(say(`create/${type}-hint`, { id: kinds[type].id() }));
      button.append(mark, element('b', null, say(`create/${type}`)), element('small', null, hint));
      button.addEventListener('click', () => search(type));
      grid.append(button);
    }
    dialog.body.replaceChildren(grid);
  };

  const search = (type) => {
    back.hidden = types.length < 2;
    dialog.setTitle(say(`create/${type}-title`));
    const word = kinds[type].id();
    const input = element('input', 'input wide-input');
    input.placeholder = spaced(say('create/search', { id: word }));
    const list = element('div', 'pick-list');
    const known = (type === 'group' ? context.answer.groups : context.answer.friends) || [];
    let picked = null;
    const add = element('button', 'btn primary', say('create/add'));
    const line = (one) =>
      [`${word} ${one.id}`, ...(one.members ? [say('venues/members', { count: one.members })] : [])].join(' · ');
    const draw = () => {
      const typed = input.value.trim();
      const lower = typed.toLowerCase();
      const hits = known
        .filter((one) => !context.listed.has(`${type}:${one.id}`))
        .filter((one) => !typed || String(one.id).includes(typed) || String(one.name || '').toLowerCase().includes(lower))
        .slice(0, SHOWN);
      const rows = hits.map((one) => row(String(one.id), one.name || String(one.id), line(one)));
      const exact = hits.some((one) => String(one.id) === typed);
      const listed = NUMBER.test(typed) && context.listed.has(`${type}:${typed}`);
      if (NUMBER.test(typed) && !exact && !listed) {
        rows.unshift(row(typed, say('create/new-row', { id: word, number: typed }), kinds[type].newHint(), true));
      }
      if (listed) list.replaceChildren(element('p', 'faint pick-none', say('create/listed')), ...rows);
      else if (typed && rows.length === 0) list.replaceChildren(element('p', 'faint pick-none', say('create/none')));
      else list.replaceChildren(...rows);
      choose(rows[0] || null);
    };
    const choose = (chosen) => {
      picked = chosen;
      for (const one of list.querySelectorAll('.pick')) one.classList.toggle('on', one === chosen);
      add.disabled = !chosen;
    };
    const row = (id, name, text, fresh = false) => {
      const one = element('div', 'pick');
      one.dataset.id = id;
      let round;
      if (fresh) {
        round = element('span', `avatar hx${type === 'group' ? ' sq' : ''}`);
        round.append(icon('i-plus'));
      } else {
        round = face(type, id, name);
      }
      const named = element('div', 'grow', name);
      named.append(element('small', null, text));
      one.append(round, named);
      one.addEventListener('click', () => choose(one));
      one.addEventListener('dblclick', () => submit());
      return one;
    };
    const submit = async () => {
      if (!picked || add.disabled) return;
      add.disabled = true;
      const id = picked.dataset.id;
      try {
        const answer = await context.request('cards.add', { value: type === 'group' ? id : `p:${id}` });
        if (answer.id) {
          dialog.close();
          onAdded(type, String(answer.id));
          return;
        }
        toast(answer.message || say('failed/save'));
      } catch {
        toast(say('failed/save'));
      }
      add.disabled = false;
    };
    input.addEventListener('input', draw);
    input.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') submit();
    });
    add.addEventListener('click', submit);
    const cancel = element('button', 'btn ghost', say('dialog/cancel'));
    cancel.addEventListener('click', () => dialog.close());
    dialog.foot.hidden = false;
    dialog.foot.replaceChildren(element('span', 'grow'), cancel, add);
    dialog.body.replaceChildren(input, element('p', 'faint pick-hint', kinds[type].searchHint()), list);
    draw();
    input.focus();
  };

  if (types.length === 1) search(types[0]);
  else pick();
}
