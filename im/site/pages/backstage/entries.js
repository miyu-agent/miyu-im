// 插件的后台（施工 I-102，项目主人 11-02：不用去找哪些插件有后台）：开着的插件声明的后台页（面板 `page`，施工 I-9 下）和整页
// （`ui.pages`，施工 I-8 下），一个一项，面板在前。侧栏「插件后台」照它列，会话浮窗「管理插件」照它给「后台」直达。

/** 插件 `plugin` 的后台：[{ page（整页的那一项，面板是空的）, href }]；关着的插件没有。 */
export function backstages(plugin) {
  if (!plugin.enabled) return [];
  const id = encodeURIComponent(plugin.package);
  const all = [];
  if (plugin.page) all.push({ page: null, href: `#backstage/${id}/panel` });
  for (const page of plugin.ui?.pages || []) all.push({ page, href: `#backstage/${id}/page/${encodeURIComponent(page.id)}` });
  return all;
}
