## 施工单 I-148：换钉，转发口带回核心拒绝里的 data

状态：做完了（2026-11-05）。

### 目的

设计 09 的转发口要把核心拒绝里的 `data` 带回页面，后面几页都要用：
- 配置冲突时现在的值；
- 写错时给人看的那一句和几处错；
- 传图续传从哪接着。

核心的客户端库 `miyu_client::open::Refused` 原来只留原因、原话，`data` 丢了。核心 CA-2（661c295d4）给它加了 `pub data: Value`。

### 怎么走

1. 核心的钉从 7783ce2c0 换到 661c295d4（CA-2）。中间进了 PK-2、PK-4、SH-2、RL-4、E-8补，im 一处不用改就编过。`package.list` 的依赖带 `>=` 碰不到 im：im 只读 `package`、`removed`。
2. 转发口核心拒了的交回 `{"refused": {"reason", "message", "data"}}`（`site/core_call.rs`）。核心以外的几种（连不上、断开）`data` 是 null。
3. 页面 `shell/core.js` 的 `refusalText` 先用核心照连接语言给的 `data.message`，再用 `data.problems` 的几句，再是原话。传图的续传（I-146 的 `images.js`）照 `data.received` 接着传，这一步起才真用得上。
4. 设计 09 的施工表插进这一步，后面顺延：供应商 I-149、添加供应商 I-150、默认模型和模型池 I-151。

### 验收

- 本机门禁、三台 CI 全绿。
- 新测试 `tests/site_core_call.rs`：真核心上 `persona.list` 转得过去，`session.send` 拒 `not_allowed`，`config.set` 写错 `expect` 回 `config_conflict` 且带 `data.current`。去掉带 data 的那一格时这条测试报红。
