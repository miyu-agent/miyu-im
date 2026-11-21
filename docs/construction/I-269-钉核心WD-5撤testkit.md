## 施工单 I-269：核心钉换成 WD-5，撤掉临时的 testkit

状态：做完了（2026-11-21），不发版。

### 起因

core1 转达：核心 WD-4、WD-5 把给人看的字整体清理成标准的软件短句（核心字表、出厂软件包的字表、拒绝原因表、命令行文案），FX-7 把 `Human::with_dir` 恢复成正式接口。im 换钉一次，同时撤掉 I-267 临时打开的 miyu-store `testkit`；断言了核心原话的测试照两张对照表（核心仓库 `internal/construction/WD-4-核心给人看的字表清理.md`、`WD-5-拒绝原因和命令行文案清理.md`）修改。

### 改了什么

- 核心钉 `36bd3c4e4` → `62d294acd`，刷了 Cargo.lock 和第三方许可证清单。
- 去掉 `crates/miyu-im/Cargo.toml` 的 `miyu-store = { workspace = true, features = ["testkit"] }`，依赖方向的测试（`crates/miyu-im-app/tests/dependencies.rs`）恢复成宿主只依赖 `miyu-chat`、`miyu-client`。

### 查到的

- 换钉以后整套测试一次全过（1570 个），没有要按对照表修改的断言。逐句核对过：im 测试里断言的核心回执（「已清空上下文。」「已全部停止。」「上下文为空。」「记忆不可用。」「当前工作区：{cwd}」）两版之间没有变化；`/remember` 的回执从「记下了：{text}」改成「已添加记忆：{text}」，im 的测试里「记下了」只出现在 `expect` 的说明里，不是断言回执。
- WD-5 点名的「文件已变」「文件已不在」「没有这个文件」「改了 1 处」在 im 里只出现在注释里，im 不抄这些话。

### 怎么验

- 门禁八项；三台 CI。
- 沙箱实测（`~/.cache/miyu-remake/wd5-live`，`run.sh`；自己的数据根，假 NapCat，真模型）：
  - 钉的核心 `62d294acd`（核心仓库在这个提交的检出编的 debug）：im 装上、启用，核心日志没有 `extension stopped`。私聊里依次发 `/clear`、一句问候、叫她读一个不存在的文件、`/remember 喜欢猫`、`/clear`、`/stop`。她回了话，调了 `read`，并转述了工具结果；核心的回执依次是「上下文为空。」「记忆不可用。」「已清空上下文。」「已全部停止。」。
  - 同样的包和剧本换成主人本机装的 rc2.4 核心，结果相同。所以清单里的核心下限 `2026.11.0-rc.2.4` 不用改。
