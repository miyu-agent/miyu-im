// 「＋ 添加供应商」（设计 09，施工 I-151；照网页软件 add-provider.js 和 onboard/flow.js 的设置页那一种）：一个浮窗里接一家供应商的几屏。
// 选一家（已配好的、常用的、本机的服务、搜索整份目录、自定义、手动填写）→ 一家（密钥、测试连接）或者自定义（ID、显示名、地址、接口、
// 密钥）→ 保存，只写这一家。纯逻辑在 add-logic.js。
//
// - 主按钮只有一个（2026-10-10 项目主人）：没测过、改了框的写「测试连接」，测着写「正在测试…」，测成了写「保存」；没测成的原话下面两个
//   按钮（2026-10-24）：「重新设置」光标放进密钥框，「跳过」不测了照填的存。没测成以后主按钮也是「跳过」。
// - 测成了上面填的几行收成一行摘要，点「修改」再摊开（2026-10-23）。核心要模型名的多一行填模型名。
// - 自定义的 ID 照地址的主机名起好填上，人改过以后不再跟着地址变（2026-10-13：能自己起）；显示名可留空（2026-10-23）。
// - 「手动填写…」关上浮窗、开原来那张表（provider-form.js）。
import { element, icon, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { secretName } from './config.js';
import { logoElement } from './bits.js';
import { cleanUrl, customId, customIdProblem, failureOf, mineRows, providerRows, setupChanges, testParams, usableIds, validUrl } from './add-logic.js';

/** 搜索整份目录：停下多久再搜（毫秒）、最多几家。 */
const SEARCH_WAIT = 200;
const SEARCH_LIMIT = 50;

/** 开浮窗：存好了调 `done(id)`，「手动填写…」调 `manual()`。 */
export function openAdd(request, done, manual) {
  const dialog = openDialog(say('models/add-provider'));
  dialog.body.classList.add('persona-body');
  const known = { featured: [], detect: null, providers: [], drivers: [], configured: new Set(), usable: new Set(), failed: null };
  let stale = null;
  let current = null;

  // 画一屏：标题、下面一行、内容；底下「上一步」或「取消」和主按钮。
  const show = (screen, back = false) => {
    current = screen;
    dialog.setTitle(screen.title);
    const parts = [];
    if (screen.sub) parts.push(element('p', 'faint', screen.sub));
    parts.push(...screen.body);
    const error = element('p', 'persona-note');
    error.hidden = true;
    parts.push(error);
    dialog.swap(parts, back);
    const leave = element('button', 'btn');
    if (screen.back) leave.append(icon('i-left'), element('span', null, say('add/back')));
    else leave.textContent = say('dialog/cancel');
    leave.addEventListener('click', () => (screen.back ? screen.back() : dialog.close()));
    const main = screen.next ? element('button', 'btn primary') : null;
    if (main) {
      main.addEventListener('click', async () => {
        if (!screen.next.ready() || main.disabled) return;
        main.disabled = true;
        error.hidden = true;
        const why = await screen.next.run();
        if (why) {
          error.textContent = why;
          error.hidden = false;
        }
        sync();
      });
    }
    screen.sync = () => sync();
    const sync = () => {
      if (!main || current !== screen) return;
      main.disabled = !screen.next.ready();
      main.textContent = screen.next.label();
    };
    sync();
    dialog.foot.replaceChildren(leave, element('span', 'grow'), ...(main ? [main] : []));
    if (screen.focus) queueMicrotask(() => screen.focus.focus());
  };
  const ready = () => current?.sync?.();

  const load = async () => {
    const quiet = (promise) => promise.catch(() => null);
    try {
      const [featured, detect, models, schema] = await Promise.all([
        core(request, 'provider.catalog', { featured: true }),
        quiet(core(request, 'provider.detect')),
        quiet(core(request, 'model.list')),
        quiet(core(request, 'config.schema')),
      ]);
      const providers = models?.providers ?? [];
      const driver = (schema?.items ?? []).find((one) => one.key === 'providers.<id>.driver');
      Object.assign(known, {
        featured: featured?.providers ?? [],
        detect,
        providers,
        configured: new Set(providers.map((one) => one.id)),
        usable: usableIds(providers),
        drivers: (driver?.options ?? []).map((one) => ({ value: one.value, name: one.name ?? one.value })),
        failed: null,
      });
    } catch (error) {
      known.failed = refusalText(error);
    }
  };

  // 选一家。
  const list = () => {
    if (known.failed) return { title: say('add/title-list'), sub: say('add/sub-list'), body: [element('p', 'persona-note', say('add/load-failed', { reason: known.failed }))] };
    const { common, local } = providerRows(known.featured, known.detect, known.usable);
    const mine = mineRows(known.providers, [...common, ...local]);
    const body = [];
    const group = (title, rows) => {
      if (title) body.push(element('div', 'add-group', say(title)));
      const box = element('div', 'add-rows');
      box.append(...rows);
      body.push(box);
    };
    if (mine.length) group('add/mine', mine.map(rowButton));
    group(mine.length ? 'add/common' : null, common.map(rowButton));
    if (local.length) group('add/local', local.map(rowButton));
    group('add/other', [
      linkRow('add/search-more', null, () => show(search())),
      linkRow('add/custom', 'add/custom-hint', () => show(custom())),
      linkRow('add/manual', 'add/manual-hint', () => {
        dialog.close();
        manual();
      }),
    ]);
    return { title: say('add/title-list'), sub: say('add/sub-list'), body };
  };

  // 选一家的一行：图标、名字、本机的写地址，右边标记、箭头；不支持的灰着。
  const rowButton = (row) => {
    const tag = row.configured
      ? ['good', say('add/tag-configured')]
      : row.kind === 'catalog' && !row.supported
        ? ['off', say('add/tag-unsupported')]
        : row.kind === 'catalog' && row.env
          ? ['', say('add/tag-found')]
          : null;
    const button = element('button', 'add-row');
    button.disabled = row.kind === 'catalog' && !row.supported;
    if ('logo' in row) button.append(logoElement(row.logo, row.name));
    button.append(element('span', 'add-row-name', row.name));
    if (row.kind === 'local') button.append(element('span', 'faint', row.host));
    if (tag) button.append(element('span', `chip ${tag[0]}`, tag[1]));
    button.append(icon('i-right'));
    button.addEventListener('click', () => show(provider(row)));
    return button;
  };
  const linkRow = (name, hint, act) => {
    const button = element('button', 'add-row');
    button.append(element('span', 'add-row-name', say(name)));
    if (hint) button.append(element('span', 'faint', say(hint)));
    button.append(icon('i-right'));
    button.addEventListener('click', act);
    return button;
  };

  // 搜索整份目录。
  const search = () => {
    const input = element('input', 'input persona-input');
    input.placeholder = say('add/search-hint');
    const out = element('div', 'add-rows');
    let timer = 0;
    let asked = 0;
    const run = async () => {
      const turn = ++asked;
      let got;
      try {
        got = await core(request, 'provider.catalog', { query: input.value.trim(), limit: SEARCH_LIMIT });
      } catch (error) {
        if (turn === asked) out.replaceChildren(element('p', 'persona-note', refusalText(error)));
        return;
      }
      if (turn !== asked) return;
      const { common } = providerRows(got?.providers ?? [], known.detect, known.usable);
      out.replaceChildren(...(common.length ? common.map(rowButton) : [element('p', 'faint', say('add/no-match'))]));
    };
    input.addEventListener('input', () => {
      clearTimeout(timer);
      timer = setTimeout(run, SEARCH_WAIT);
    });
    run();
    return { title: say('add/search-title'), body: [input, out], back: () => show(list(), true), focus: input };
  };

  // 一家：密钥、测试连接。
  const provider = (row) => {
    const found = row.configured ? known.providers.find((one) => one.id === row.configured) : null;
    const target = row.configured
      ? { kind: 'configured', id: row.configured, driver: found?.driver ?? null, base_url: found?.base_url ?? null }
      : row.kind === 'local'
        ? { kind: 'local', catalog: row.id }
        : { kind: 'catalog', catalog: row.id, env: row.env };
    let keyInput = null;
    let keyRow = null;
    if (target.kind === 'configured') {
      keyRow = field('add/key', element('span', null, say('add/key-configured')));
    } else if (target.kind === 'catalog') {
      keyInput = keyField(target.env ? say('add/key-env', { name: target.env }) : say('add/key-paste'));
      keyRow = field(null, keyInput);
    }
    const key = () => {
      const typed = keyInput?.value.trim() ?? '';
      if (typed) return { value: typed };
      return target.kind === 'catalog' && target.env ? { env: target.env } : null;
    };
    const canTest = () => target.kind !== 'catalog' || Boolean(key());
    // 「重新设置」：配好了的一家「已配置」换成密码框，贴一个新的密钥再测。
    const rekey = () => {
      if (target.kind === 'configured' && !keyInput && keyRow) {
        keyInput = keyField(say('add/key-paste'));
        keyRow.replaceChildren(keyInput);
      }
      return keyInput;
    };
    const auto = canTest() && !keyInput?.value;
    return testing(target, row.name, keyRow ? [keyRow] : [], key, canTest, keyInput, { auto, pasted: () => Boolean(keyInput?.value.trim()), rekey });
  };

  // 自定义：ID、显示名、地址、接口、密钥。
  const custom = () => {
    const url = element('input', 'input persona-input');
    url.placeholder = 'https://…';
    const drivers = known.drivers.length ? known.drivers : ['openai-chat', 'anthropic', 'openai-responses'].map((value) => ({ value, name: value }));
    const driver = element('select', 'select');
    for (const one of drivers) driver.append(new Option(one.name, one.value));
    driver.addEventListener('change', () => stale?.());
    const keyInput = keyField(say('add/key-optional'));
    const idInput = element('input', 'input persona-input');
    idInput.placeholder = say('add/id-auto');
    const idError = element('p', 'persona-note');
    idError.hidden = true;
    let idTyped = false;
    const checkId = () => {
      const why = customIdProblem(idInput.value.trim(), known.configured);
      idError.hidden = !why || !idInput.value.trim();
      idError.textContent = why ? say(`models/form-${why}`, { id: idInput.value.trim() }) : '';
      return !why;
    };
    idInput.addEventListener('input', () => {
      idTyped = true;
      checkId();
      ready();
    });
    const nameInput = element('input', 'input persona-input');
    nameInput.placeholder = say('add/name-hint');
    url.addEventListener('input', () => {
      if (!idTyped) {
        const typed = cleanUrl(url.value);
        idInput.value = validUrl(typed) ? customId(typed, known.configured) : '';
        checkId();
      }
      stale?.();
    });
    const target = () => ({ kind: 'custom', driver: driver.value, base_url: cleanUrl(url.value), id: idInput.value.trim(), name: nameInput.value.trim() || null });
    const idBox = element('div');
    idBox.append(idInput, idError);
    const rows = [field('models/form-id', idBox), field('add/name', nameInput), field('add/url', url), field('add/driver', driver), field('add/key', keyInput)];
    const key = () => (keyInput.value.trim() ? { value: keyInput.value.trim() } : null);
    const canTest = () => validUrl(cleanUrl(url.value)) && checkId();
    return testing(target, '', rows, key, canTest, idInput, { auto: false, pasted: () => Boolean(keyInput.value.trim()), title: say('add/custom-title'), rekey: () => keyInput });
  };

  // 密钥框：不回显，改了要重测。
  const keyField = (hint) => {
    const input = element('input', 'input persona-input');
    Object.assign(input, { type: 'password', placeholder: hint, autocomplete: 'off' });
    input.addEventListener('input', () => stale?.());
    return input;
  };

  // 测试连接、保存那一段（一家、自定义共用）。
  const testing = (targetOf, name, fields, keyOf, canTest, first, opts) => {
    const target = () => (typeof targetOf === 'function' ? targetOf() : targetOf);
    const modelInput = element('input', 'input persona-input');
    modelInput.placeholder = say('add/model-name');
    const modelRow = field('add/model-name', modelInput);
    modelRow.hidden = true;
    const result = element('div', 'add-result');
    result.hidden = true;
    const state = { ok: null, running: false, gone: false, failed: false };
    const canRun = () => !state.running && canTest() && (modelRow.hidden || Boolean(modelInput.value.trim()));
    stale = () => {
      state.failed = false;
      fold(false);
      if (state.ok) {
        state.ok = null;
        result.hidden = true;
      }
      ready();
    };
    modelInput.addEventListener('input', () => {
      state.failed = false;
      ready();
    });
    const skip = () => save(target(), keyOf(), opts.pasted(), name);
    const skipHere = async () => {
      const why = await skip();
      if (why) result.replaceChildren(element('p', 'persona-note', why));
    };
    const again = () => {
      state.failed = false;
      fold(false);
      result.hidden = true;
      const box = opts.rekey?.() ?? first;
      ready();
      queueMicrotask(() => box?.focus());
    };
    const run = async () => {
      if (!canTest()) return;
      Object.assign(state, { running: true, ok: null, failed: false });
      result.hidden = false;
      result.className = 'add-result faint';
      result.replaceChildren(element('span', null, name ? say('add/test-busy', { name }) : say('add/test-busy-plain')));
      ready();
      let got;
      try {
        got = await core(request, 'provider.test', testParams(target(), keyOf(), modelRow.hidden ? null : modelInput.value.trim()));
      } catch (error) {
        got = { ok: false, stage: 'request', error: { class: 'other', message: refusalText(error) } };
      }
      if (state.gone) return;
      state.running = false;
      if (got?.ok) {
        state.ok = got;
        fold(true);
        result.className = 'add-result good';
        result.replaceChildren(icon('i-check'), element('span', null, say('add/test-ok', { count: new Set(got.models ?? []).size })));
      } else {
        const why = failureOf(got);
        state.failed = true;
        result.className = 'add-result bad';
        const line = element('div', 'add-result-line');
        line.append(icon('i-x'), element('span', null, say(why.key) + (why.status ? ` · ${why.status}` : '')));
        const actions = element('div', 'add-fail');
        const redo = element('button', 'btn', say('add/redo'));
        redo.addEventListener('click', again);
        const skipButton = element('button', 'btn', say('add/skip'));
        skipButton.addEventListener('click', skipHere);
        actions.append(redo, skipButton);
        result.replaceChildren(line, ...(why.message ? [element('p', 'faint', why.message)] : []), actions);
        if (why.needsModel) {
          modelRow.hidden = false;
          queueMicrotask(() => modelInput.focus());
        }
      }
      ready();
    };
    for (const box of [...fields, modelRow]) {
      box.addEventListener('keydown', (event) => {
        if (event.key !== 'Enter' || event.isComposing || state.ok) return;
        event.preventDefault();
        if (state.failed) skipHere();
        else if (canRun()) run();
      });
    }
    // 测成以后上面几行收成的那一行：这一家的名字或 ID、地址，右边「修改」。
    const summary = element('div', 'add-summary');
    summary.hidden = true;
    const fold = (on) => {
      if (fields.length < 2) return;
      for (const box of fields) box.hidden = on;
      summary.hidden = !on;
      if (!on) return;
      const now = target();
      const what = now.kind === 'custom' ? [now.name || now.id, now.base_url].filter(Boolean).join(' · ') : name;
      const edit = element('button', 'btn ghost', say('add/edit'));
      edit.addEventListener('click', () => fold(false));
      summary.replaceChildren(element('span', null, what), edit);
    };
    if (opts.auto) queueMicrotask(run);
    return {
      title: opts.title ?? name,
      body: [summary, ...fields, modelRow, result],
      back: () => {
        state.gone = true;
        stale = null;
        show(list(), true);
      },
      next: {
        label: () => (state.ok ? say('dialog/save') : state.failed ? say('add/skip') : say(state.running ? 'add/testing' : 'add/test')),
        ready: () => (state.ok ? true : state.failed || canRun()),
        run: async () => {
          if (state.ok) return save(target(), keyOf(), opts.pasted(), name);
          if (state.failed) return skip();
          await run();
          return null;
        },
      },
      focus: first,
    };
  };

  // 存：贴了密钥的先存成新名字的密钥，再一条 config.set 写进系统配置；交回出错的字。
  const save = async (target, key, pasted, name) => {
    try {
      const id = setupChanges(target, null, known.configured).id;
      let ref = key?.env ? { env: key.env } : null;
      if (pasted && key?.value) {
        const secret = secretName(id, Date.now());
        await core(request, 'secret.set', { name: secret, value: key.value });
        ref = { secret };
      }
      const { changes } = setupChanges(target, ref, known.configured);
      if (changes.length) await core(request, 'config.set', { layer: 'system', changes });
      stale = null;
      dialog.close();
      done(id, name);
      return null;
    } catch (error) {
      return say('add/save-failed', { reason: refusalText(error) });
    }
  };

  dialog.body.append(element('p', 'faint', say('add/loading')));
  load().then(() => show(list()));
}

/** 一行：上面名字（null 的没有），下面控件。 */
function field(label, control) {
  const box = element('div', 'persona-field');
  if (label) box.append(element('span', 'persona-label', say(label)));
  box.append(control);
  return box;
}
