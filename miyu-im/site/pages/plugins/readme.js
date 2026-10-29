// 插件详情「介绍」（施工 I-9 上）：插件带的 README 照 Markdown 的常用写法画成页面元素。只认标题、段落、列表、引用、分隔线、代码块，
// 行内的代码、粗体、斜体、链接；字一律经 textContent 放（不经 innerHTML），链接只收 http、https，新窗口开。别的写法照字画。

/** 把 Markdown 的 `text` 画成一串元素。 */
export function readme(text) {
  const out = [];
  const lines = text.replace(/\r\n?/g, '\n').split('\n');
  let at = 0;
  while (at < lines.length) {
    const line = lines[at];
    if (/^\s*$/.test(line)) {
      at += 1;
    } else if (/^```/.test(line)) {
      const code = [];
      at += 1;
      while (at < lines.length && !/^```/.test(lines[at])) code.push(lines[at++]);
      at += 1;
      const pre = document.createElement('pre');
      pre.append(node('code', code.join('\n')));
      out.push(pre);
    } else if (/^#{1,6}\s/.test(line)) {
      const level = line.match(/^#+/)[0].length;
      out.push(inline(document.createElement(`h${Math.min(level + 2, 6)}`), line.replace(/^#+\s+/, '').replace(/\s+#+\s*$/, '')));
      at += 1;
    } else if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
      out.push(document.createElement('hr'));
      at += 1;
    } else if (/^\s*([-*+]|\d+[.)])\s/.test(line)) {
      const ordered = /^\s*\d/.test(line);
      const list = document.createElement(ordered ? 'ol' : 'ul');
      while (at < lines.length && /^\s*([-*+]|\d+[.)])\s/.test(lines[at])) {
        list.append(inline(document.createElement('li'), lines[at].replace(/^\s*([-*+]|\d+[.)])\s+/, '')));
        at += 1;
      }
      out.push(list);
    } else if (/^>/.test(line)) {
      const quoted = [];
      while (at < lines.length && /^>/.test(lines[at])) quoted.push(lines[at++].replace(/^>\s?/, ''));
      out.push(inline(document.createElement('blockquote'), quoted.join(' ')));
    } else {
      const paragraph = [];
      while (at < lines.length && !/^\s*$/.test(lines[at]) && !/^(```|#{1,6}\s|>|\s*([-*+]|\d+[.)])\s)/.test(lines[at])) {
        paragraph.push(lines[at++].trim());
      }
      out.push(inline(document.createElement('p'), paragraph.join(' ')));
    }
  }
  return out;
}

/** 行内的写法：`代码`、**粗体**、*斜体*、[字](地址)。照先后一段一段放进 `parent`。 */
function inline(parent, text) {
  const pattern = /`([^`]+)`|\*\*([^*]+)\*\*|\*([^*]+)\*|\[([^\]]+)\]\(([^)\s]+)\)/g;
  let last = 0;
  for (const found of text.matchAll(pattern)) {
    parent.append(text.slice(last, found.index));
    const [, code, bold, italic, label, href] = found;
    if (code !== undefined) parent.append(node('code', code));
    else if (bold !== undefined) parent.append(node('strong', bold));
    else if (italic !== undefined) parent.append(node('em', italic));
    else parent.append(link(label, href));
    last = found.index + found[0].length;
  }
  parent.append(text.slice(last));
  return parent;
}

/** 一个链接：只收 http、https 的地址，别的照字画。 */
function link(label, href) {
  if (!/^https?:\/\//i.test(href)) return document.createTextNode(label);
  const anchor = node('a', label);
  anchor.href = href;
  anchor.target = '_blank';
  anchor.rel = 'noopener noreferrer';
  return anchor;
}

/** 一个 `tag` 元素，里面是字 `text`。 */
function node(tag, text) {
  const made = document.createElement(tag);
  made.textContent = text;
  return made;
}
