// 记一笔、改一笔的浮窗（施工 S-1 下，设计 03 第一节第 6 条）：平台、号、名字、金额、币种、备注；改的那一笔没换算的多一格汇率，
// 「查汇率」照现在的默认币种问一次（查不到的自己填）。删要点两次。改、删只在这里，不给 AI。
import { h, say, money, day, platformName, bareId, reasonOf } from './ui.js';

/** 开浮窗：`entry` 是要改的那一笔（空的是记一笔），`base` 是默认币种。 */
export function openEntry(ui, { entry, base }) {
  const editing = entry != null;
  const field = (label, input, hint) => h('label', { class: 'field' }, h('span', { text: label }), input, hint && h('small', { text: hint }));
  const platform = h('select', { disabled: editing },
    ...(editing ? [entry.platform] : ui.platforms()).map((one) => h('option', { value: one, text: platformName(one) })));
  const user = h('input', { inputmode: 'numeric', value: editing ? bareId(entry.user) : '', placeholder: say('id_hint') });
  const name = h('input', { value: entry?.name ?? '', placeholder: say('name_hint') });
  const amount = h('input', { type: 'number', min: '0', step: 'any', value: entry?.amount ?? '' });
  const currency = h('input', { class: 'currency', maxlength: '3', value: entry?.currency ?? base });
  const note = h('input', { value: entry?.note ?? '' });
  const failed = h('p', { class: 'note error' });

  const pendingRate = editing && (entry.converted == null || entry.base !== base);
  const rate = h('input', { type: 'number', min: '0', step: 'any', value: '' });
  const lookup = h('button', { class: 'button', type: 'button', text: say('look_up_rate'), onclick: async () => {
    lookup.disabled = true;
    try {
      const got = await ui.call('rate', { currency: currency.value.trim() || entry.currency });
      if (got.rate != null) rate.value = String(got.rate);
      else failed.textContent = say('rate_not_found');
    } catch (error) {
      failed.textContent = reasonOf(error);
    }
    lookup.disabled = false;
  } });

  const body = h('div', { class: 'form' },
    field(say('platform'), platform),
    h('div', { class: 'pair' }, field(say('id'), user), field(say('name'), name)),
    h('div', { class: 'pair' }, field(say('amount'), amount), field(say('currency'), currency, say('currency_hint', { base }))),
    field(say('note_label'), note));
  if (pendingRate) {
    body.append(field(say('rate', { currency: entry.currency, base }), h('div', { class: 'inline' }, rate, lookup), say('rate_hint')));
  } else if (editing && entry.currency !== base && entry.rate != null) {
    body.append(h('p', { class: 'frozen', text: say('frozen', { rate: entry.rate, currency: entry.currency, base, value: money(entry.converted) }) }));
  }
  if (editing) {
    body.append(h('p', { class: 'facts', text: [say('recorded_on', { day: day(entry.at) }), entry.by ? say('recorded_by', { who: bareId(entry.by) }) : null].filter(Boolean).join(' · ') }));
  }

  const save = h('button', { class: 'button primary', type: 'button', text: say(editing ? 'save' : 'add') });
  const cancel = h('button', { class: 'button', type: 'button', text: say('cancel'), onclick: () => dialog.close() });
  let armed = false;
  const remove = editing && h('button', { class: 'button danger', type: 'button', text: say('delete'), onclick: async () => {
    if (!armed) {
      armed = true;
      remove.textContent = say('delete_again');
      return;
    }
    remove.disabled = true;
    try {
      await ui.call('delete', { id: entry.id });
      dialog.close();
      ui.toast(say('deleted'));
      await ui.reload();
    } catch (error) {
      remove.disabled = false;
      failed.textContent = reasonOf(error);
    }
  } });

  save.addEventListener('click', async () => {
    failed.textContent = '';
    const params = {};
    const typed = { user: user.value.trim(), name: name.value.trim(), note: note.value.trim(), currency: currency.value.trim().toUpperCase() };
    const value = Number(amount.value);
    if (!(value > 0)) { failed.textContent = say('bad_amount'); return; }
    if (!/^[A-Z]{3}$/.test(typed.currency)) { failed.textContent = say('bad_currency'); return; }
    if (typed.user && !/^\d+$/.test(typed.user)) { failed.textContent = say('bad_id'); return; }
    if (!typed.user && !typed.name) { failed.textContent = say('need_who'); return; }
    if (editing) {
      params.id = entry.id;
      if (typed.user !== bareId(entry.user) && typed.user) params.user = typed.user;
      if (typed.name !== (entry.name ?? '') && typed.name) params.name = typed.name;
      if (typed.note !== (entry.note ?? '') && typed.note) params.note = typed.note;
      if (value !== entry.amount) params.amount = value;
      if (typed.currency !== entry.currency) params.currency = typed.currency;
      if (pendingRate && Number(rate.value) > 0) params.rate = Number(rate.value);
    } else {
      Object.assign(params, { platform: platform.value, amount: value, currency: typed.currency });
      for (const key of ['user', 'name', 'note']) if (typed[key]) params[key] = typed[key];
    }
    save.disabled = true;
    try {
      await ui.call(editing ? 'update' : 'add', params);
      dialog.close();
      ui.toast(say(editing ? 'saved' : 'added'));
      await ui.reload();
    } catch (error) {
      save.disabled = false;
      failed.textContent = reasonOf(error);
    }
  });

  const dialog = h('dialog', { class: 'sheet entry' },
    h('h2', { text: say(editing ? 'edit_title' : 'add_title') }),
    body, failed,
    h('div', { class: 'actions' }, remove, h('span', { class: 'spacer' }), cancel, save));
  dialog.addEventListener('close', () => dialog.remove());
  document.body.append(dialog);
  dialog.showModal();
}
