//! 命令行印给人看的字的几种写法（施工 I-87，照核心设计 47「命令行文案」）：报错 `miyu im: <消息>`，警告 `miyu im: 警告：<消息>`，
//! 建议另起一行 `提示：<能照着做的>`（前缀照这时说的语言，字在 `data/human/*.json` 的 `prefix`、`warning`、`hint`）；帮助页一个
//! 命令一份（`help/<命令>`），不认识的命令、写错的参数照这里说。

use super::Texts;

impl Texts {
    /// 报错：`<程序>: <消息>`。
    pub fn error(&self, body: &str) -> String {
        format!("{}: {body}", self.say("prefix", &[]))
    }

    /// 警告：`<程序>: 警告：<消息>`。
    pub fn warning(&self, body: &str) -> String {
        format!(
            "{}: {}{body}",
            self.say("prefix", &[]),
            self.say("warning", &[])
        )
    }

    /// 一行提示：`提示：<内容>`。
    pub fn hint(&self, body: &str) -> String {
        format!("{}{body}", self.say("hint", &[]))
    }

    /// 报错，下面每个 `hints` 一行提示。
    pub fn hinted(&self, body: &str, hints: &[String]) -> String {
        hints.iter().fold(self.error(body), |said, hint| {
            format!("{said}\n{}", self.hint(hint))
        })
    }

    /// 警告，下面每个 `hints` 一行提示。
    pub fn warned(&self, body: &str, hints: &[String]) -> String {
        hints.iter().fold(self.warning(body), |said, hint| {
            format!("{said}\n{}", self.hint(hint))
        })
    }

    /// 帮助页：`command` 没写的是总表，写了的是那个命令的（`venue show` 照 `venue`）。认不出的命令交回空的。
    pub fn help(&self, command: Option<&str>) -> Option<String> {
        let key = match command {
            None => "help/main",
            Some(
                name @ ("start" | "stop" | "restart" | "status" | "web" | "logs" | "venue"
                | "whitelist" | "access"),
            ) => {
                return Some(self.say(&format!("help/{name}"), &[]));
            }
            Some(_) => return None,
        };
        Some(self.say(key, &[]))
    }

    /// 没写命令。
    pub fn no_command(&self) -> String {
        self.hinted(
            &self.say("cli/no-command", &[]),
            &[self.say("cli/see-help", &[])],
        )
    }

    /// 不认识的命令 `command`。
    pub fn unknown_command(&self, command: &str) -> String {
        self.hinted(
            &self.say("cli/unknown", &[("command", command.to_string())]),
            &[self.say("cli/see-help", &[])],
        )
    }

    /// 命令 `command` 的参数写错了。
    pub fn bad_args(&self, command: &str) -> String {
        let fields = [("command", command.to_string())];
        self.hinted(
            &self.say("cli/bad-args", &fields),
            &[self.say("cli/see-command-help", &fields)],
        )
    }
}
