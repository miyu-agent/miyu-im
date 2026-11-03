## 施工单 I-125：提示里的换端口命令去掉 --system

状态：做完了（2026-11-03；core1 在 FH-7 第三轮演练里发现）。

### 目的

核心改回只给一个人用以后（10-28），`miyu config set`、`miyu pkg install` 都没有 `--system` 这个选项了，带上它会报「无法识别的选项」。im 端口被占时的两句提示（NapCat 端口、后台页端口）还教人敲 `miyu config set --system …`，照着敲就报错。

### 怎么走

1. 给人看的字（`miyu-im/data/human/` 中、英、日三份）：`failure/port-in-use-hint`、`notice/page-in-use-hint` 里的命令去掉 `--system`。
2. 核心的钉换到 main 的新头（核心修好了 FH-3 合进去以后红的测试，产品代码没动）。
3. 别处（蓝图、作者说明、代码）查过没有 `--system`；以前的施工单是当时的记录，不改。

### 验收

- 本机门禁、三台 CI 全绿。
- 端口被占时印出来的命令照着敲，核心认得。
