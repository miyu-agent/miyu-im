## 施工单 I-178：`miyu im web` 发现 im 没开就先开，再打开网页

状态：做完了（2026-11-09；项目主人 todolist「发版前的check」第 7 条，core1 转来）。

### 为什么

im 没开时跑 `miyu im web`，原来只说一句「接入通讯平台未运行，用 'miyu im start' 启动」就退出。人要的是打开网页，开 im 是顺带的一步，命令自己能做。

### 怎么改

1. 连核心照 `miyu im start` 的样子：核心没在跑就拉起（原来只连在跑的）。
2. im 的开关关着的照 `start` 调 `extension.enable`，先说一句「已启动接入通讯平台」；开着却停下了的照 `restart` 再拉起。开着、正在起或在等着再拉起的不动它，照常等。
3. 等它的页面服务听上（状态文件写着端口、进程号对得上），最多 30 秒，听上了照原来要票、开浏览器。中途停下了的照 `miyu im status` 说为什么（端口被占这类，带标准错误的最后几行）；30 秒还没听上的说「没能按时起来，用 'miyu im status' 查看原因」。都是退出码 1、不开浏览器。
4. 核心那边没有 im 这个包的，照 `status` 说没有。
5. 帮助 `miyu im web --help` 加一句：没启动的先启动它（同 start）。开关打开以后和 `start` 一样，核心下次启动也会启动它。

技术取舍：核心的客户端连接（`miyu_client::open::Core`）只有一问一答，收不到 `extension.changed` 推送，所以等的时候隔 100 毫秒问一次 `extension.status`、读一次状态文件。这是一次性的命令、有 30 秒上限，不常驻；以后客户端能收推送了换成订阅 `extensions`。

### 验收

- `web::when_im_is_off_it_starts_it_then_opens_the_page`：起好 im、`miyu im stop` 关掉，再跑 `web`，印「已启动接入通讯平台」、开的网址是页面服务的端口。
- `web::when_im_cannot_start_it_says_why`：NapCat 的端口被测试占着，`web` 打开开关以后 im 停下，说「已停止」和「端口 … 已被占用」，不开浏览器。
- 把「打开开关」那一步拿掉，上面两条都报红（等 30 秒后说没能按时起来）。
- 本机门禁、三台 CI 全绿。
