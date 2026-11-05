// 人格的详情（设计 09，施工 I-145；照网页软件的 persona-editor.js）：浮窗里最上面三行外观（施工 I-146，appearance.js，换了当场存），
// 下面名字、简单描述、具体提示词、示范对话、人设提醒短语、子代理可选。改完点「保存」一起发一条 persona.set（写一半不生效），提示词带 persona.read 给的版本；「取消」关掉、丢掉没存的。
//
// - 2026-10-08 项目主人（网页软件定的，这里照守）：来自哪一层、以谁为底、编号都不露；说明能清空（发空字，不发 unset）。
// - 改了没存的点底、Esc、关闭都不关（长的字一按就丢太亏），提示先保存或取消；存好了提示一句、关掉（10-17）。
// - 别处改过了（persona_conflict）写一句、给「重新读」；别的拒绝写核心的原话。
// - 删除照核心给的 remove：改过的出厂「恢复出厂」、自己建的「删除」，先弹窗问（10-20）。出厂人格删了是连包卸掉，删前删后各读一次
//   包表，多卸掉的那个写进提示。
// - 网页软件这里还有「记忆 N 条 · 查看」：im 网页没有记忆页，不放。
import { element, say } from '../../shell/i18n.js';
import { confirmFirst, openDialog, toast } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { area, cleanPairs, halfPair, pairsEditor } from './pairs.js';
import { appearance } from './appearance.js';

/** 开人格 `id` 的详情：`name` 是列表里写的（读完以前标题先写它），存了调 `hooks.saved()`，删了调 `hooks.removed(remains)`；
 *  `fresh` 是刚建的，读完焦点放到具体提示词。 */
export function openPersona(request, id, name, hooks, fresh = false) {
  let before = null;
  let remove = null;
  let fields = null;
  // 外观当场存过：「保存」也亮着，点了提示已保存、关掉（2026-10-13 项目主人）。
  let touched = false;
  const dialog = openDialog(name || say('personas/untitled'), () => look.dispose(), () => {
    if (!dirty()) return true;
    note(say('personas/unsaved'));
    return false;
  });
  dialog.body.classList.add('persona-body');
  const message = element('p', 'persona-note');
  message.hidden = true;
  const note = (text, ...more) => {
    message.hidden = !text;
    message.replaceChildren(element('span', null, text), ...more);
  };
  const look = appearance(request, id, {
    saved: () => {
      touched = true;
      sync();
      hooks.saved();
    },
    say: (text) => note(text),
  });
  const save = element('button', 'btn primary', say('dialog/save'));
  const cancel = element('button', 'btn', say('dialog/cancel'));
  cancel.addEventListener('click', () => dialog.close());
  save.addEventListener('click', () => store());

  const current = () => ({
    ...before,
    name: fields.name.value,
    summary: fields.summary.value,
    persona: fields.persona.value,
    reminders: fields.reminders.value,
    pairs: fields.pairs.value(),
    subagent: fields.subagent,
  });
  const dirty = () => before !== null && fields !== null && personaSave(before, current()) !== null;
  const sync = () => {
    save.disabled = !(dirty() || touched) || !fields.name.value.trim();
  };

  const load = async () => {
    const read = (prompt) => core(request, 'persona.read', { persona: id, prompt });
    let got;
    let texts;
    try {
      [got, ...texts] = await Promise.all([
        core(request, 'persona.get', { persona: id }),
        read('persona'),
        read('reminders'),
        read('examples'),
      ]);
    } catch (error) {
      dialog.body.replaceChildren(element('p', 'persona-note', refusalText(error)));
      return;
    }
    const [persona, reminders, examples] = texts;
    remove = got.remove || null;
    look.set(got);
    before = {
      name: typeof got.name === 'string' ? got.name : '',
      summary: typeof got.summary === 'string' ? got.summary : '',
      persona: persona.text || '',
      reminders: reminders.text || '',
      pairs: Array.isArray(examples.pairs) ? examples.pairs : [],
      subagent: got.subagent === true,
      versions: { persona: persona.version ?? null, reminders: reminders.version ?? null, examples: examples.version ?? null },
    };
    draw();
    if (fresh) fields.persona.focus();
  };

  const draw = () => {
    dialog.setTitle(before.name || say('personas/untitled'));
    const name = element('input', 'input persona-input');
    name.value = before.name;
    const summary = element('input', 'input persona-input');
    summary.value = before.summary;
    summary.placeholder = say('personas/summary-hint');
    const persona = area(before.persona, say('personas/persona-hint'), 6);
    const reminders = area(before.reminders, say('personas/reminders-hint'), 2);
    const them = () => name.value.trim() || say('personas/them');
    const pairs = pairsEditor(before.pairs, them, sync);
    const subagent = element('button', before.subagent ? 'switch on' : 'switch');
    subagent.append(element('i'));
    fields = { name, summary, persona, reminders, pairs, subagent: before.subagent };
    subagent.addEventListener('click', () => {
      fields.subagent = !fields.subagent;
      subagent.classList.toggle('on', fields.subagent);
      sync();
    });
    name.addEventListener('input', () => {
      pairs.rename(them());
      sync();
    });
    for (const box of [summary, persona, reminders]) box.addEventListener('input', sync);
    dialog.body.replaceChildren(
      ...look.rows.map(([label, box]) => field(label, box, 'div')),
      field('personas/name', name),
      field('personas/summary', summary),
      field('personas/persona', persona),
      field('personas/examples', pairs.el),
      field('personas/reminders', reminders),
      switchRow('personas/subagent', subagent),
      message,
    );
    dialog.foot.replaceChildren(...(remove ? [removeButton()] : []), element('span', 'grow'), cancel, save);
    sync();
  };

  const store = async () => {
    const half = halfPair(fields.pairs.value());
    if (half >= 0) {
      note(say('personas/half-pair', { n: half + 1 }));
      fields.pairs.mark(half);
      return;
    }
    const payload = personaSave(before, current());
    if (!payload) {
      // 只改了外观：已经当场存了，这里说一声、关掉。
      if (touched) {
        toast(say('dialog/saved'));
        dialog.close();
      }
      return;
    }
    save.disabled = true;
    try {
      await core(request, 'persona.set', { persona: id, ...payload });
    } catch (error) {
      sync();
      if (error.reason === 'persona_conflict') {
        const again = element('button', 'btn ghost', say('personas/reload'));
        again.addEventListener('click', () => load());
        note(say('personas/conflict'), again);
      } else {
        note(refusalText(error));
      }
      return;
    }
    toast(say('dialog/saved'));
    hooks.saved();
    dialog.close();
  };

  const removeButton = () => {
    const button = element('button', 'btn ghost danger', say(`personas/${remove}`));
    button.addEventListener('click', async () => {
      const shown = before.name || id;
      const asked = await confirmFirst(say(`personas/${remove}`), say(`personas/${remove}-ask`, { name: shown }), say(`personas/${remove}`));
      if (asked) drop();
    });
    return button;
  };

  const drop = async () => {
    const packages = () => core(request, 'package.list').then((got) => got.packages || null, () => null);
    const listed = remove === 'delete' ? await packages() : null;
    try {
      const got = await core(request, 'persona.delete', { persona: id });
      const gone = !got.remains && listed ? newlyRemoved(listed, await packages()) : null;
      if (gone) toast(say('personas/deleted-package', { name: gone.name || gone.package }));
      else toast(say(got.remains ? 'personas/restored' : 'personas/deleted'));
      hooks.removed(Boolean(got.remains));
      if (got.remains) load();
      else dialog.close();
    } catch (error) {
      note(refusalText(error));
    }
  };

  load();
}

/** 一格：上面名字、下面控件（`tag` 是外层的元素：里面有好几个按钮的用 div，不用 label）。 */
function field(label, control, tag = 'label') {
  const box = element(tag, 'persona-field');
  box.append(element('span', 'persona-label', say(label)), control);
  return box;
}

/** 开关一行：左边名字、右边开关。 */
function switchRow(label, control) {
  const box = element('div', 'persona-switch');
  box.append(element('span', 'persona-label', say(label)), control);
  return box;
}

/** `before` 读进来的、`after` 现在写的：交回 persona.set 要带的 { changes?, prompts? }，没改的是 null（纯逻辑，同网页软件的
 *  personaSave）。 */
export function personaSave(before, after) {
  const out = {};
  const changes = [];
  const name = after.name.trim();
  if (name && name !== before.name) changes.push({ key: 'persona.name', value: name });
  if (after.summary.trim() !== before.summary.trim()) changes.push({ key: 'persona.summary', value: after.summary.trim() });
  if (Boolean(after.subagent) !== Boolean(before.subagent)) changes.push({ key: 'persona.subagent', value: Boolean(after.subagent) });
  if (changes.length) out.changes = changes;
  const prompts = {};
  for (const key of ['persona', 'reminders']) {
    if (after[key].trim() !== before[key].trim()) prompts[key] = { text: after[key].trim(), expect: before.versions[key] };
  }
  const pairs = cleanPairs(after.pairs);
  if (JSON.stringify(pairs) !== JSON.stringify(cleanPairs(before.pairs))) {
    prompts.examples = { pairs, expect: before.versions.examples };
  }
  if (Object.keys(prompts).length) out.prompts = prompts;
  return out.changes || out.prompts ? out : null;
}

/** 删人格连带卸掉的出厂包：删前 `listed`、删后 `after` 两份包表里，删后多出来的 removed 的那一个（纯逻辑）。 */
export function newlyRemoved(listed, after) {
  if (!Array.isArray(listed) || !Array.isArray(after)) return null;
  const was = new Set(listed.filter((one) => one?.removed).map((one) => one.package));
  return after.find((one) => one?.removed && !was.has(one.package)) ?? null;
}
