// 插件面板（施工 I-9 下，设计 01 第四节第 5 条、第九节第 5 条；照核心 package-pages.md「框和网页之间怎么说」、WebUI 的
// backstage/frame.js）：有后台页的插件在沙箱框里打开（网址 #plugins/<编号>/panel）。框占满这一页，上面一条窄的：返回详情、插件的名字。
//
// - 框：sandbox 只许脚本、表单，不给 allow-same-origin（框里的页面在空的来源里跑，读不到这一页的登录）；地址是宿主给的带票据的
//   /p/<票据>/<编号>/（plugins.panel）。
// - 框第一次载入完，造一个 MessageChannel，一头交给框（只交这一次：框后来再载入，这一页分不出载入的是谁，后台页要做成单页）；以后
//   照 JSON-RPC 说话，认的一律是这个插件，框说自己是谁不算数。方法：context、settings.get、settings.set、call（经宿主的
//   plugins.settings.*、plugins.call）；推送：theme.changed（这一页换了深浅）、settings.changed（框自己存了设置以后）。
// - context 多一格 im（施工 P-1，设计 01 第四节第 5 条）：从 im 打开的写平台、这个 im 连着的机器人号（号、名字、连着没有）。同一份
//   后台页在网页的「软件后台」里打开时没有这一格，页面照它分是从哪边打开的、要不要按连接分。
import { element, icon, say } from '../../shell/i18n.js';
import { named } from '../venues/plugins.js';

/** 带给框的几个颜色：这一页的样式变量 → 交出去的名字（同 WebUI）。 */
const COLORS = [
  ['--accent', 'accent'],
  ['--surface', 'surface'],
  ['--surface-2', 'surface_2'],
  ['--text', 'text'],
  ['--text-soft', 'text_soft'],
  ['--line', 'line'],
  ['--danger', 'danger'],
];

/** 插件 `plugin` 的面板：交回 { parts（要放进页面的元素）, close() }。`context` 是 { request, platform, connections() }。 */
export function drawPanel(plugin, { request, platform, connections }) {
  const bar = element('div', 'panel-bar');
  const back = element('a', 'btn ghost');
  back.href = `#plugins/${encodeURIComponent(plugin.package)}`;
  back.append(icon('i-left'), document.createTextNode(named(plugin.name) || plugin.package));
  bar.append(back);
  const frame = element('iframe', 'plug-frame');
  frame.setAttribute('sandbox', 'allow-scripts allow-forms');
  frame.setAttribute('allow', 'clipboard-write');
  frame.title = named(plugin.name) || plugin.package;
  const failed = element('p', 'faint');
  failed.hidden = true;
  const host = new FrameHost(request, plugin.package, frame, () => opened(platform, connections()));
  request('plugins.panel', { package: plugin.package }).then(
    (answer) => {
      frame.src = answer.url;
    },
    () => {
      failed.hidden = false;
      failed.textContent = say('plugins-page/panel-failed');
      frame.hidden = true;
    },
  );
  return { parts: [bar, failed, frame], close: () => host.close() };
}

/** 框和这一页之间的那条通道（模块的说明）。 */
class FrameHost {
  constructor(request, packageId, frame, im) {
    this.request = request;
    this.id = packageId;
    this.im = im;
    this.port = null;
    let handed = false;
    frame.addEventListener('load', () => {
      if (handed || !frame.src) return;
      handed = true;
      const channel = new MessageChannel();
      this.port = channel.port1;
      this.port.onmessage = (event) => this.handle(event.data);
      frame.contentWindow?.postMessage({ miyu: 'port' }, '*', [channel.port2]);
    });
    this.lookSig = JSON.stringify(look());
    const changed = () => {
      const now = look();
      const sig = JSON.stringify(now);
      if (sig === this.lookSig) return;
      this.lookSig = sig;
      this.push('theme.changed', now);
    };
    this.watch = new MutationObserver(changed);
    this.watch.observe(document.documentElement, { attributes: true });
    this.media = matchMedia('(prefers-color-scheme: dark)');
    this.onMedia = changed;
    this.media.addEventListener('change', this.onMedia);
  }

  /** 收掉：通道关了，不再看深浅。 */
  close() {
    this.port?.close();
    this.port = null;
    this.watch.disconnect();
    this.media.removeEventListener('change', this.onMedia);
  }

  push(method, params) {
    this.port?.postMessage({ jsonrpc: '2.0', method, params });
  }

  reply(id, result, error) {
    if (id === undefined || id === null || !this.port) return;
    this.port.postMessage(error ? { jsonrpc: '2.0', id, error } : { jsonrpc: '2.0', id, result });
  }

  /** 框发来的一条。 */
  async handle(message) {
    if (!message || typeof message !== 'object' || message.jsonrpc !== '2.0' || typeof message.method !== 'string') {
      this.reply(message?.id ?? null, null, { code: -32600, message: 'Invalid Request' });
      return;
    }
    const params = message.params ?? {};
    try {
      if (message.method === 'context') {
        this.reply(message.id, { package: this.id, language: document.documentElement.lang, ...look(), im: this.im() });
      } else if (message.method === 'settings.get') {
        this.reply(message.id, await this.request('plugins.settings.get', { package: this.id }));
      } else if (message.method === 'settings.set') {
        const answer = await this.request('plugins.settings.set', { package: this.id, changes: params.changes });
        this.reply(message.id, answer);
        const keys = Object.keys(answer?.keys ?? {});
        if (keys.length > 0) this.push('settings.changed', { keys });
      } else if (message.method === 'call') {
        const args = { package: this.id, method: params.method };
        if (params.params !== undefined) args.params = params.params;
        this.reply(message.id, await this.request('plugins.call', args));
      } else {
        this.reply(message.id, null, { code: -32601, message: 'Method not found' });
      }
    } catch (reason) {
      this.reply(message.id, null, { code: -32000, message: String(reason), data: { reason: String(reason) } });
    }
  }
}

/** 交给框的 `im` 那一格（模块的说明）：平台、机器人号（照外壳问到的连接）。 */
function opened(platform, connections) {
  const bots = (connections || []).map((one) => ({ id: String(one.bot), name: one.name || null, connected: one.connected !== false }));
  return { platform, bots };
}

/** 这一页这时的深浅和几个颜色（照算出来的颜色）。 */
function look() {
  const style = getComputedStyle(document.body);
  const probe = document.createElement('i');
  document.body.append(probe);
  const colors = Object.fromEntries(
    COLORS.map(([variable, name]) => {
      probe.style.color = style.getPropertyValue(variable).trim() || 'inherit';
      return [name, getComputedStyle(probe).color];
    }),
  );
  probe.remove();
  const rgb = (colors.surface.match(/[\d.]+/g) ?? ['255', '255', '255']).slice(0, 3).map(Number);
  const light = (0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]) / 255 > 0.5;
  return { theme: light ? 'light' : 'dark', colors };
}
