// 接入通讯平台页面的字（施工 I-5 上）：三种语言，照网址片段的 lang、浏览器的语言挑（app.js）。字段写 {名字}。
//
// 写法要守（同后台页的 texts.js）：`export const TEXTS = ` 后面到最后的 `;` 是一段 JSON：不写注释、不写结尾的逗号，键用双引号。
// 测试（crates/miyu-im-onebot/tests/page.rs）照它读，比三种语言的键。
export const TEXTS = {
  "en": {
    "failed/core": "Can't reach Miyu",
    "failed/network": "Can't reach Connect Messaging",
    "failed/ticket": "Link expired",
    "signed-in": "Signed in: {account}",
    "signed-out": "Not signed in. Open from the web app or the terminal.",
    "signing-in": "Signing in…",
    "sign-out": "Sign out",
    "title": "Connect Messaging"
  },
  "ja": {
    "failed/core": "Miyu に接続できません",
    "failed/network": "メッセージング接続に接続できません",
    "failed/ticket": "リンクの期限が切れています",
    "signed-in": "ログイン済み：{account}",
    "signed-out": "未ログイン。Web アプリかターミナルから開いてください。",
    "signing-in": "ログインしています…",
    "sign-out": "ログアウト",
    "title": "メッセージング接続"
  },
  "zh": {
    "failed/core": "连不上 Miyu",
    "failed/network": "连不上接入通讯平台",
    "failed/ticket": "链接已失效",
    "signed-in": "已登录：{account}",
    "signed-out": "未登录。从网页软件或终端打开。",
    "signing-in": "正在登录…",
    "sign-out": "登出",
    "title": "接入通讯平台"
  }
};
