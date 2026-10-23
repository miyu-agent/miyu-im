// 赞助记录的后台页（设计 03 第一节第 6 条，施工 S-1 下；核心 docs/blueprint/package-pages.md「框和网页之间怎么说」）：通道、字、颜色，
// 页顶一排（标题、记一笔），下面头上的合计和排行榜（board.js）、明细（ledger.js），记一笔、改一笔的浮窗（edit.js）。
//
// - 页面跑在网页给的隔离框里：只经网页交来的 MessageChannel 照 JSON-RPC 说，方法 context、call（转 package.call → 包的
//   method.call）；推送 theme.changed。
// - 不轮询：改了什么当场重读；框重新看得见时（visibilitychange）重读一次，接住她这期间记的。
import { h, say, useLanguage, reasonOf } from './ui.js';
import { boardView } from './board.js';
import { ledgerView } from './ledger.js';
import { openEntry } from './edit.js';

(() => {
  const COLORS = {
    accent: '--accent', surface: '--surface', surface_2: '--surface-2', text: '--text', text_soft: '--text-soft',
    line: '--line', danger: '--danger',
  };
  const app = /** @type {HTMLElement} */ (document.getElementById('app'));
  let channel = null;
  let board = null;
  let ledger = null;

  class Channel {
    constructor(port) {
      this.port = port;
      this.n = 0;
      this.waiting = new Map();
      this.handlers = {};
      port.onmessage = (event) => this.receive(event.data);
    }

    receive(message) {
      if (!message || typeof message !== 'object') return;
      if (message.id == null) {
        if (typeof message.method === 'string') this.handlers[message.method]?.(message.params ?? {});
        return;
      }
      const waiter = this.waiting.get(message.id);
      if (!waiter) return;
      this.waiting.delete(message.id);
      if (!message.error) return waiter.resolve(message.result);
      const { message: text, code, data } = message.error;
      waiter.reject(Object.assign(new Error(text ?? String(code)), { code, data: data ?? null }));
    }

    call(method, params) {
      return new Promise((resolve, reject) => {
        const id = ++this.n;
        this.waiting.set(id, { resolve, reject });
        this.port.postMessage(params === undefined ? { jsonrpc: '2.0', id, method } : { jsonrpc: '2.0', id, method, params });
      });
    }

    on(method, handler) { this.handlers[method] = handler; }
  }

  function look(theme, colors) {
    const root = document.documentElement;
    root.dataset.theme = theme === 'dark' ? 'dark' : 'light';
    for (const [name, variable] of Object.entries(COLORS)) {
      const color = colors?.[name];
      if (typeof color === 'string' && color) root.style.setProperty(variable, color);
      else root.style.removeProperty(variable);
    }
  }

  function centered(text, again) {
    app.className = 'boot';
    app.replaceChildren(h('p', { class: again ? 'error' : 'boot-wait', text }),
      again && h('button', { class: 'button primary', type: 'button', text: say('retry'), onclick: again }));
  }

  /** 调包的方法 `method`（经网页的 call 转 package.call）。 */
  const call = (method, params) => channel.call('call', params === undefined ? { method } : { method, params });

  /** 底下冒一句、过一会儿收起。 */
  function toast(text) {
    const shown = h('div', { class: 'toast', text });
    document.body.append(shown);
    requestAnimationFrame(() => shown.classList.add('show'));
    setTimeout(() => { shown.classList.remove('show'); setTimeout(() => shown.remove(), 200); }, 2000);
  }

  const ui = {
    call, toast,
    /** 改了什么之后：明细、合计、排行榜都重读（明细先读：排行榜的平台下拉照明细里见过的平台）。 */
    reload: () => ledger.load().then(() => board.load()),
    /** 点一条：开改一笔的浮窗。 */
    open: (entry, base) => openEntry(ui, { entry, base }),
    /** 头上「待补汇率」：明细只看没换算的。 */
    pending: () => ledger.onlyPending(),
    /** 明细里出现过的平台（筛的下拉、记一笔的平台照它）。 */
    platforms: () => ledger.platforms(),
  };

  function shell() {
    board = boardView(ui);
    ledger = ledgerView(ui);
    const add = h('button', {
      class: 'button primary', type: 'button', text: say('add'),
      onclick: () => openEntry(ui, { entry: null, base: board.base() }),
    });
    const bar = h('header', { class: 'bar' }, h('h1', { text: say('title') }), h('span', { class: 'spacer' }), add);
    app.className = 'shell';
    app.replaceChildren(bar, h('main', { class: 'page' }, board.el, ledger.el));
  }

  async function start() {
    centered(say('loading'));
    try {
      const context = await channel.call('context');
      useLanguage(context?.language);
      document.documentElement.lang = context?.language ?? '';
      document.title = say('title');
      look(context?.theme, context?.colors);
      shell();
      await ui.reload();
    } catch (error) {
      centered(say('failed', { reason: reasonOf(error) }), start);
    }
  }

  window.addEventListener('message', (event) => {
    if (channel || event.source !== window.parent || event.data?.miyu !== 'port' || !event.ports?.[0]) return;
    channel = new Channel(event.ports[0]);
    channel.on('theme.changed', (params) => look(params.theme, params.colors));
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible' && board) ui.reload().catch(() => {});
    });
    start();
  });
  centered(say('loading'));
})();
