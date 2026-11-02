// 会话卡上的小标签（施工 I-7 上，设计 01 第八节第 3 条「几个说明它怎么设的小标签」）：施工 I-111（项目主人 11-02：卡上只放人格、模型、
// 预设这三样重要的，别的没意义）起只写这三样。卡上写了的照常画；没写、跟着下面几层的（会话默认值、全局）淡一档画出来，一样都没有的
// 不写。是否回复看卡上的开关。
import { say } from '../../shell/i18n.js';

/** `one`（model.js 的一张卡）的标签：[{ text, inherited }]；`personas`、`presets` 是编号到名字。 */
export function chips(one, personas, presets) {
  const shown = [];
  const add = (key, label, names) => {
    const mine = one.set[key];
    const value = mine ?? one.defaults[key]?.value;
    if (value == null || value === '') return;
    shown.push({ text: say(label, { name: names?.get(value) || value }), inherited: mine == null });
  };
  add('persona', 'chip/persona', personas);
  add('model', 'chip/model');
  add('preset', 'chip/preset', presets);
  return shown;
}
