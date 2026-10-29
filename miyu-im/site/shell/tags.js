// 字的列表的控件（2026-10-20 项目主人验收：触发关键词、管理的人、回执撤回命令、插件设置里的列表都用它）：一个一个小标签，× 去掉一个，
// 最后一格填了加一个。
//
// - 回车加；输入法组字时的回车（isComposing、keyCode 229）是选字，不算。离开这一格时框里还有字的也加上。
// - 加了、去了的当场画上（不等宿主答），再交 `save(新的列表)`；宿主答了重画的照它。原来只靠宿主重画：答得慢、答不成的，页面上框里
//   的字还在、也不变成标签（「识人映射回车没用」）。
// - 没写、正照继承的画淡的标签；在继承的列表上加、去（不是只剩新加的那一个），之后照写了的画。
// - 一样的不重复加。
import { element, icon, say } from './i18n.js';

/**
 * 画一组标签。`options`：
 * - items：写了的列表；没写（照继承）的是 null。
 * - inherited：继承来的列表（items 是 null 时画它）。
 * - shown(item)：标签上的字（不写的照原样）。
 * - normalize(字)：填的字换成存的那一项，交回空的不加（不写的照去掉两头空白）。
 * - placeholder：最后一格的占位字。
 * - save(list)：改了的整个列表。
 */
export function tagInput({ items, inherited, shown = String, normalize = (text) => text, placeholder, save }) {
  const box = element('div', 'tags');
  let own = items != null;
  let list = [...(own ? items : inherited || [])];
  const adding = element('input', 'input tag-add');
  adding.placeholder = placeholder || say('field/add-word');
  adding.enterKeyHint = 'done';

  const draw = () => {
    const tags = list.map((item, at) => {
      const tag = element('span', own ? 'tag' : 'tag inherited', shown(item));
      const remove = element('button');
      remove.type = 'button';
      remove.title = say('field/remove');
      remove.append(icon('i-x'));
      remove.addEventListener('click', () => change(list.filter((_, index) => index !== at)));
      tag.append(remove);
      return tag;
    });
    // 最后一格不拿下来再放回去：拿下来会丢了焦点，接着填的要再点一下。
    for (const old of [...box.children]) if (old !== adding) old.remove();
    if (adding.parentNode !== box) box.append(adding);
    adding.before(...tags);
  };
  const change = (next) => {
    list = next;
    own = true;
    draw();
    save([...list]);
  };
  const commit = () => {
    const typed = adding.value.trim();
    if (typed === '') return false;
    const item = normalize(typed);
    adding.value = '';
    if (item === '' || item == null) return false;
    if (list.some((one) => JSON.stringify(one) === JSON.stringify(item))) return false;
    change([...list, item]);
    return true;
  };
  adding.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter' || event.isComposing || event.keyCode === 229) return;
    event.preventDefault();
    commit();
  });
  adding.addEventListener('blur', commit);
  draw();
  return box;
}
