// 接入通讯平台页面的字（施工 I-5 上）：三种语言，照网址片段的 lang、浏览器的语言挑（app.js）。字段写 {名字}。
//
// 写法要守（同后台页的 texts.js）：`export const TEXTS = ` 后面到最后的 `;` 是一段 JSON：不写注释、不写结尾的逗号，键用双引号。
// 测试（crates/miyu-im-onebot/tests/page.rs）照它读，比三种语言的键。
export const TEXTS = {
  "en": {
    "failed/core": "Can't reach Miyu",
    "failed/network": "Can't reach Connect Messaging",
    "failed/password": "Wrong username or password",
    "failed/throttled": "Too many attempts. Try again in {seconds}s",
    "failed/ticket": "Link expired",
    "password": "Password",
    "signed-in": "Signed in: {account}",
    "signed-out": "You can also open it from the web app or the terminal.",
    "signing-in": "Signing in…",
    "sign-in": "Sign in",
    "sign-out": "Sign out",
    "title": "Connect Messaging",
    "user": "Username"
  },
  "ja": {
    "failed/core": "Miyu に接続できません",
    "failed/network": "メッセージング接続に接続できません",
    "failed/password": "ユーザー名またはパスワードが違います",
    "failed/throttled": "試行回数が多すぎます。{seconds} 秒後にもう一度お試しください",
    "failed/ticket": "リンクの期限が切れています",
    "password": "パスワード",
    "signed-in": "ログイン済み：{account}",
    "signed-out": "Web アプリやターミナルから開くこともできます。",
    "signing-in": "ログインしています…",
    "sign-in": "ログイン",
    "sign-out": "ログアウト",
    "title": "メッセージング接続",
    "user": "ユーザー名"
  },
  "zh": {
    "failed/core": "连不上 Miyu",
    "failed/network": "连不上接入通讯平台",
    "failed/password": "用户名或密码错误",
    "failed/throttled": "尝试次数过多，{seconds} 秒后再试",
    "failed/ticket": "链接已失效",
    "password": "密码",
    "signed-in": "已登录：{account}",
    "signed-out": "也可以从网页软件或终端直接打开。",
    "signing-in": "正在登录…",
    "sign-in": "登录",
    "sign-out": "登出",
    "title": "接入通讯平台",
    "user": "用户名"
  }
};
