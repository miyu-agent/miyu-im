// 选人（设计 11 G4，施工 I-168）：要填人的地方先选平台、再填号码，号码框照见过的人提示（浏览器自己的下拉，`<datalist>`），不再手写
// `qq:10002`。使用限制页加人用整个控件，标签框（跳过判断的用户、管理员命令用户）只用提示（`suggest`）。样子在 shell/person-pick.css。
import { element, say } from './i18n.js';
import { platform } from '../pages/home/platforms.js';

/** 提示表的编号：一页上可能有几个。 */
let made = 0;

/** 给输入框 `input` 挂上见过的人的提示：`people` 是 [{ id: 号, name? }]，选的是号，名字写在后面。交回提示表（要放进页面里）。 */
export function suggest(input, people) {
  const list = element('datalist');
  list.id = `people-${++made}`;
  for (const one of people) {
    const option = document.createElement('option');
    option.value = String(one.id);
    if (one.name) option.label = one.name;
    list.append(option);
  }
  input.setAttribute('list', list.id);
  return list;
}

/**
 * 选一个人：`platforms` 是挂着的平台（不止一个时先选平台），`chosen` 是先选着的那一个，`people` 是见过的人 [{ id: `平台:号`, name? }]，
 * 加的时候调 `pick(平台:号)`，按钮上写 `label`。号是空的不加。交回元素。
 */
export function personPick({ platforms, chosen, people, label, pick }) {
  const box = element('div', 'person-pick');
  let on = platforms.includes(chosen) ? chosen : platforms[0] || '';
  const number = element('input', 'input person-number');
  number.placeholder = say('person-pick/number');
  number.inputMode = 'numeric';
  const hints = element('span', 'person-hints');
  const refill = () => {
    const mine = people.filter((one) => String(one.id).startsWith(`${on}:`)).map((one) => ({ id: String(one.id).slice(on.length + 1), name: one.name }));
    hints.replaceChildren(suggest(number, mine));
  };
  if (platforms.length > 1) {
    const select = element('select', 'select');
    for (const one of platforms) select.append(new Option(platform(one).name(), one));
    select.value = on;
    select.addEventListener('change', () => {
      on = select.value;
      refill();
    });
    box.append(select);
  }
  const add = element('button', 'btn', label);
  add.type = 'button';
  const commit = () => {
    const typed = number.value.trim();
    if (!on || typed === '') return;
    number.value = '';
    pick(`${on}:${typed}`);
  };
  add.addEventListener('click', commit);
  number.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.isComposing && event.keyCode !== 229) commit();
  });
  refill();
  box.append(number, hints, add);
  return box;
}
