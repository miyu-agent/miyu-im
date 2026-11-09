## 施工单 I-183 下中：Telegram 连接在后台页上新建、改、删，连接卡和概况标签

状态：做完了（2026-11-09；照设计 07 第八节 8.1，前端是项目主人当天看过 demo 定的）。

### 为什么

I-183 中下以后 Telegram 驱动能收发文字，但连接表只能手改配置：后台页的新建连接、连接设置都是照 QQ 的端口表写的，Telegram 一个连接是一个令牌（加一份代理），不开端口。demo 定了页面的样子（8.1），这一步把它接上真的宿主。

### 怎么改

1. 宿主（`miyu-im`）：
   - 驱动多两件：`token_table()`（连接表是令牌表的报出令牌那一列和另外几列，每列带「没交时写的值」；默认没有）、`probe(params, config)`（照令牌、代理当场查一次；默认答做不到）。
   - `site/token_table.rs`：驱动报了令牌表的，`connection.add`、`connection.edit`、`connection.remove` 走这里，另加 `connection.check`（照表里那一个现在的令牌再查）。新建先查、查成了令牌存成密钥再写表；同一个机器人再加答 `bot_taken`；改令牌查出别的机器人答 `other_bot`；写成了删掉不再引用的旧密钥。
   - `site/reported.rs`：连接卡带上驱动报的 `index`、`username`、隐私模式（连着且开着的 `trouble` 写 `privacy`）；还没认出号的连接（令牌一开始就不对）也出一张卡，照第几个认。
   - 群卡片概况多 `hearing`（`Link::hearing`，默认没有）：收不全消息的群、话题群，页面在「概况」标题右边放小标签。
2. Telegram 驱动：
   - `probe.rs`：getMe，交 `{bot, name, username, privacy}`；查不成的答 `bad_token`、`unreachable`、`bad_proxy`、`bad_params`。
   - 新见到的群在旁边问一次 getChatMember（机器人是不是管理员），记在 `known` 里；`hearing` 照隐私模式、管理员、话题群答。
   - 代理「跟随联网设置」存 `env`（空的也认）。
3. 网页：新建连接的 Telegram 几步（令牌加代理一屏、检查结果、你的 Telegram ID、完成页写 @用户名）、连接设置（机器人只读、令牌、代理、隐私模式能重新检查）、首页连接卡的几种毛病（别处也在收、令牌失效、连不上、代理不对、没令牌、隐私模式）、消息记录组名写 @用户名、概况标签；`pages/home/telegram.js` 放两处共用的代理三选一、检查结果、隐私模式提醒；中、英、日三份字。

### 坑

- 核心的一列字（`im.tg.proxies`）不收空字：交 `["direct", ""]` 整个 `config.set` 答 `config_invalid`、什么都不存。所以「跟随联网设置」不能存空的，存 `env`；令牌表的每一列带「没交时写的值」，补齐短了的一列、页面交空的都写它。集成测试改代理那一步第一趟就是红在这里。
- QQ 的端口表不变：驱动没报令牌表的照旧走原来的 `connection.*`。

### 验收

- 集成测试 `telegram_page`：真核心、真程序、本机假 Bot API，经网页通道：令牌不对答 `bad_token`；对的交回查到的、连接表多一项、驱动照它连上、卡上带用户名和隐私模式；再加答 `bot_taken`；重新检查；改代理（交空白的存 `env`，驱动换上）；删了连接表空。
- 单元测试：令牌表的增、改、删、补列（`token_table/tests.rs`）；连接卡并上驱动报的几格（`reported/tests.rs`）；代理写法读回写出一样、空的认成跟随。
- 本机门禁、三台 CI 全绿。
