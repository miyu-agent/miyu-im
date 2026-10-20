//! `miyu-im` 在标准错误上说给人听的字（`onebot.md` 第一条「样子」「出错」「给人看的字」）：字放在包文件夹的
//! `data/human/{zh,en,ja}.json`（施工 S-6 上从资源目录的 `software/onebot/` 挪来，[`Package`]），核心那一份照 [`Human::load`]
//! 从资源目录读，桥的这一份照 [`Human::with_dir`] 补读，说法的编号照旧是 `software/onebot/<哪一句>`。这里只管挑哪一句、换进什么字段。`start`、`stop`、`restart`、`status`、`logs` 说的
//! （施工 O-18）、`venue show` 说的和场所规则的问题说成话（施工 O-21）、限流满了发进群里的那一句（施工 O-23）、`web` 找不到
//! `miyu` 的那一句（施工 O-28 补）也在这里。
//!
//! 说话的语言：握手以前照系统的语言（[`system_language`]；施工 O-20 起桥不读配置，不看 `ui.language`），握手以后照核心回的
//! `language`。三种语言各写一份（`ja.json` 施工 O-45 起是日文，原来照英文写）。

use miyu_chat::{Entry, Problem, Source};
use miyu_client::places::ResourceRoot;
use miyu_client::protocol::Said;
use miyu_client::settings::problem::Code;
use miyu_client::texts::{Human, HumanError};

use crate::control::{Halt, Report};
use crate::logs::Heading;
use crate::package::Package;
use crate::rules::{Card, key_of};
use crate::serve::{Failure, Notice};
use crate::venue::Shown;

/// 这个包的说法编号的前缀：软件包的说法照它原来在资源目录里的位置起（`store/resources.md`「怎么走」第 3 条第 4 款）；字挪进
/// 包文件夹以后编号照旧（施工 S-6 上）。
const KEYS: &str = "software/onebot";

/// 握手以前说话的语言：照系统的语言 `locale`（`miyu_client::places::locale`），`zh`、`ja` 开头的照它，别的、没有的说英文。和核心
/// 照握手的 `locale` 算 `ui.language = auto` 的是同一个规矩（「施工时定的」第 44 条）。
pub fn system_language(locale: Option<&str>) -> &'static str {
    match locale {
        Some(locale) if locale.starts_with("zh") => "zh",
        Some(locale) if locale.starts_with("ja") => "ja",
        _ => "en",
    }
}

/// 读好的一种语言的字。
#[derive(Debug, Clone)]
pub struct Texts {
    /// 资源目录：核心那一份字在这里，换语言时再读。
    resources: ResourceRoot,
    /// 包文件夹：桥自己的字在它的 `data/` 里，换语言时再读（施工 S-6 上）。
    package: Package,
    /// 现在说的语言：`zh`、`en`、`ja` 这样的写法。
    language: String,
    /// 这种语言的字。
    human: Human,
}

impl Texts {
    /// 读 `language` 这一种：核心那一份照资源目录 `resources`，桥的这一份照包文件夹 `package` 的 `data/`。
    ///
    /// # Errors
    ///
    /// 有一份给人看的字读得到却读不懂（[`Human::load`]、[`Human::with_dir`]）。读不到的不算错：那一句照实印出编号。
    pub fn load(
        resources: ResourceRoot,
        package: Package,
        language: &str,
    ) -> Result<Texts, HumanError> {
        let human = Human::load(&resources, language)?.with_dir(&package.data(), KEYS, language)?;
        Ok(Texts {
            resources,
            package,
            language: language.to_string(),
            human,
        })
    }

    /// 换成说 `language`；已经是它的不再读。
    ///
    /// # Errors
    ///
    /// 那种语言的字读不懂：还照原来的说。
    pub fn speak(&mut self, language: &str) -> Result<(), HumanError> {
        if language != self.language {
            *self = Texts::load(self.resources.clone(), self.package.clone(), language)?;
        }
        Ok(())
    }

    /// 资源目录。
    pub fn resources(&self) -> &ResourceRoot {
        &self.resources
    }

    /// 包文件夹（施工 S-6 上）：出厂数据、给模型看的字照它的 `data/` 读。
    pub fn package(&self) -> &Package {
        &self.package
    }

    /// 用法不对。
    pub fn usage(&self) -> String {
        self.say("usage", &[])
    }

    /// 运行日志装不上（照样跑）。
    pub fn no_log(&self, reason: &str) -> String {
        self.say("no-log", &[("reason", reason.to_string())])
    }

    /// 这时说话用的语言（`zh`、`en`、`ja`）：`web` 照它拼进页面的网址（施工 I-6 上）。
    pub fn language(&self) -> &str {
        &self.language
    }

    /// `web`：im 没在跑、页面服务没听上（施工 I-6 上）。
    pub fn web_not_running(&self) -> String {
        self.say("web/not-running", &[])
    }

    /// `web`：连不上核心、核心拒了，`reason` 是原话（施工 I-6 上）。
    pub fn web_failed(&self, reason: &str) -> String {
        self.say("web/failed", &[("reason", reason.to_string())])
    }

    /// `web`：网址 `url` 交给浏览器了（`opened`），或者没打开、照它自己开（施工 I-6 上）。
    pub fn web_opened(&self, url: &str, opened: bool) -> String {
        let key = if opened {
            "web/opened"
        } else {
            "web/open-by-hand"
        };
        self.say(key, &[("url", url.to_string())])
    }

    /// 起来了（令牌没设的说怎么设）、连上了、断开了。号没认出来的不说号。
    pub fn notice(&self, notice: &Notice) -> String {
        match notice {
            Notice::Listening { port, .. } => {
                self.say("notice/listening", &[("port", port.to_string())])
            }
            Notice::Connected { bot: Some(bot) } => {
                self.say("notice/connected-as", &[("bot", bot.to_string())])
            }
            Notice::Connected { bot: None } => self.say("notice/connected", &[]),
            Notice::Disconnected { bot: Some(bot) } => {
                self.say("notice/disconnected-as", &[("bot", bot.to_string())])
            }
            Notice::Disconnected { bot: None } => self.say("notice/disconnected", &[]),
            Notice::NoToken => self.say("notice/no-token", &[]),
            Notice::Page { port } => self.say("notice/page", &[("port", port.to_string())]),
            Notice::PageInUse { port } => {
                self.say("notice/page-in-use", &[("port", port.to_string())])
            }
        }
    }

    /// 起不来、停了。
    pub fn failure(&self, failure: &Failure) -> String {
        match failure {
            Failure::Core(reason) => self.say("failure/core", &[("reason", reason.clone())]),
            Failure::NotSpawned => self.say("failure/not-spawned", &[]),
            Failure::PortInUse(port) => {
                self.say("failure/port-in-use", &[("port", port.to_string())])
            }
            Failure::Crashed(reason) => self.say("failure/crashed", &[("reason", reason.clone())]),
            Failure::Start(reason) => self.say("failure/start", &[("reason", reason.clone())]),
            Failure::Factory(problems) => {
                let mut said = self.say("failure/factory", &[]);
                for problem in problems {
                    said.push_str(&format!("\n  {}", self.problem(problem)));
                }
                said
            }
        }
    }

    /// `venue show` 说的几句（施工 O-21）。
    pub fn shown(&self, shown: &Shown) -> String {
        match shown {
            Shown::None => self.say("venue/none", &[]),
            Shown::Defaults => self.say("venue/defaults", &[]),
            Shown::Problems => self.say("venue/problems", &[]),
            Shown::BadVenue(venue) => self.say("venue/bad-venue", &[("venue", venue.clone())]),
        }
    }

    /// `whitelist` 说的一句（施工 I-13）。
    pub fn whitelist(&self, said: &crate::whitelist::Said) -> String {
        use crate::whitelist::Said;
        match said {
            Said::Empty => self.say("whitelist/empty", &[]),
            Said::Bad(number) => self.say("whitelist/bad", &[("number", number.clone())]),
            Said::Added(entry) => self.say("whitelist/added", &[("entry", entry.clone())]),
            Said::Already(entry) => self.say("whitelist/already", &[("entry", entry.clone())]),
            Said::Removed(entry) => self.say("whitelist/removed", &[("entry", entry.clone())]),
            Said::Absent(entry) => self.say("whitelist/absent", &[("entry", entry.clone())]),
        }
    }

    /// 规则设到的一项（施工 O-21）：键 `key`、值照 TOML 写、来处。
    pub fn entry(&self, key: &str, entry: &Entry) -> String {
        let origin = &entry.origin;
        let from = self.place(
            origin.source,
            &origin.file,
            Some(origin.rule),
            Some(origin.line),
        );
        self.say(
            "venue/entry",
            &[
                ("key", key.to_string()),
                ("value", entry.value.toml()),
                ("from", from),
            ],
        )
    }

    /// 一条场所规则的问题说成话（施工 O-21，「施工时定的」第 56 条）：在哪，加错在哪（一种原因码一句；出厂参数缺了的是
    /// `wrong_type` 没有原文，另说一句）。
    pub fn problem(&self, problem: &Problem) -> String {
        let at = self.place(
            problem.source,
            &problem.file,
            problem.rule,
            problem.at.map(|at| at.line),
        );
        let key = ("key", problem.key.clone().unwrap_or_default());
        let got = ("got", problem.got.clone().unwrap_or_default());
        let why = ("why", problem.why.clone().unwrap_or_default());
        let what = match (problem.code, &problem.suggest) {
            (Code::Unreadable, _) => self.say("problem/unreadable", &[why]),
            (Code::TooBig, _) => self.say("problem/too-big", &[]),
            (Code::NotUtf8, _) => self.say("problem/not-utf8", &[]),
            (Code::Syntax, _) => self.say("problem/syntax", &[why]),
            (Code::UnknownKey, Some(suggest)) => {
                self.say("problem/unknown-key", &[key, ("suggest", suggest.clone())])
            }
            (Code::UnknownKey, None) => self.say("problem/unknown-key-plain", &[key]),
            (Code::WrongType, _) if problem.got.is_none() => self.say("problem/missing", &[key]),
            (Code::WrongType, _) => self.say("problem/wrong-type", &[key, got]),
            (Code::NotAnOption, _) => self.say("problem/not-an-option", &[key, got]),
            (Code::OutOfRange, _) => self.say("problem/out-of-range", &[key, got]),
            (Code::BadFormat, _) => self.say("problem/bad-format", &[key, got]),
            // 群聊内核、读文件报不出别的原因码：照原样印。
            (code, _) => self.say("problem/other", &[key, ("code", code.as_str().to_string())]),
        };
        self.say("venue/problem", &[("at", at), ("what", what)])
    }

    /// 在哪：出厂或系统、文件名，有第几条规则、第几行的带上（第几条规则只和第几行一起说）。配置项那一层（施工 O-41）只说是
    /// 哪一项（`venue/setting`）：文件是桥拼的，人看不到，行号对不上什么。
    fn place(
        &self,
        source: Source,
        file: &str,
        rule: Option<usize>,
        line: Option<usize>,
    ) -> String {
        let source = match source {
            Source::Factory => self.say("venue/factory", &[]),
            // 卡片那一层（施工 O-58 上）：说是哪一张卡，文件同样是桥拼的。
            Source::Settings => {
                return match Card::parse(file) {
                    Some(Card::Defaults) => self.say("venue/card-defaults", &[]),
                    Some(Card::OtherGroups) => self.say("venue/card-other-groups", &[]),
                    Some(Card::OtherPrivates) => self.say("venue/card-other-privates", &[]),
                    Some(Card::Group(id)) => {
                        self.say("venue/card-group", &[("id", id.to_string())])
                    }
                    Some(Card::Private(id)) => {
                        self.say("venue/card-private", &[("id", id.to_string())])
                    }
                    None => {
                        let key = rule.and_then(key_of).unwrap_or_else(|| file.to_string());
                        self.say("venue/setting", &[("key", key)])
                    }
                };
            }
            Source::System => self.say("venue/system", &[]),
        };
        let mut fields = vec![("source", source), ("file", file.to_string())];
        let key = match (rule, line) {
            (Some(rule), Some(line)) => {
                fields.extend([("rule", rule.to_string()), ("line", line.to_string())]);
                "venue/rule"
            }
            (None, Some(line)) => {
                fields.push(("line", line.to_string()));
                "venue/line"
            }
            (_, None) => "venue/file",
        };
        self.say(key, &fields)
    }

    /// `start`、`stop`、`restart`、`status` 说的一句（施工 O-18）。
    pub fn report(&self, report: &Report) -> String {
        match report {
            Report::Started => self.say("control/started", &[]),
            Report::Stopped => self.say("control/stopped", &[]),
            Report::Restarted => self.say("control/restarted", &[]),
            Report::Off => self.say("status/off", &[]),
            Report::Starting => self.say("status/starting", &[]),
            Report::Running(pid) => self.say("status/running", &[("pid", pid.to_string())]),
            Report::Waiting { seconds, failures } => self.say(
                "status/waiting",
                &[
                    ("seconds", seconds.to_string()),
                    ("failures", failures.to_string()),
                ],
            ),
            Report::Halted(halt) => self.say("status/stopped", &[("reason", self.halt(halt))]),
            Report::Stderr => self.say("status/stderr", &[]),
            Report::Other(state) => self.say("status/other", &[("state", state.clone())]),
            Report::Missing => self.say("status/missing", &[]),
            Report::Napcat {
                implementation,
                version,
                bot,
            } => self.say(
                "status/napcat",
                &[
                    ("implementation", implementation.clone()),
                    ("version", version.clone()),
                    ("bot", bot.clone()),
                ],
            ),
            Report::NapcatBot(bot) => self.say("status/napcat-bot", &[("bot", bot.clone())]),
            Report::NoNapcat => self.say("status/no-napcat", &[]),
            Report::Listen(listen) => self.say("status/listen", &[("listen", listen.to_string())]),
            Report::Page => self.say("status/page", &[]),
            Report::Misread(keys) => {
                // 键换成清单里给人看的名字（施工 O-41 补二），照这时说的语言。
                let names = crate::settings::labels(&self.package, keys, &self.language);
                self.say(
                    "status/misread",
                    &[(
                        "keys",
                        names.join(&self.say("status/misread-separator", &[])),
                    )],
                )
            }
        }
    }

    /// 停下的原因；不认识的照原样。
    fn halt(&self, halt: &Halt) -> String {
        match halt {
            Halt::ConfigError => self.say("reason/config_error", &[]),
            Halt::FailedRepeatedly(failures) => self.say(
                "reason/failed_repeatedly",
                &[("failures", failures.to_string())],
            ),
            Halt::NotInstalled => self.say("reason/not_installed", &[]),
            Halt::CannotStart => self.say("reason/cannot_start", &[]),
            Halt::ProtocolMismatch => self.say("reason/protocol_mismatch", &[]),
            Halt::Other(code) => code.clone(),
        }
    }

    /// `logs` 的标题、还没有运行日志那一句（施工 O-18）。
    pub fn heading(&self, heading: &Heading) -> String {
        match heading {
            Heading::Stderr(path) => self.say("logs/stderr", &[("path", path.clone())]),
            Heading::Log(path) => self.say("logs/log", &[("path", path.clone())]),
            Heading::None(path) => self.say("logs/none", &[("path", path.clone())]),
        }
    }

    /// 说 `key` 那一句，换进 `fields`。换不出来的（没有这一句、少了字段：是 bug，`tests/texts.rs` 守着）不吞掉：
    /// 照实印出说法的编号和字段，再记一行运行日志。
    fn say(&self, key: &str, fields: &[(&str, String)]) -> String {
        let said = fields.iter().fold(
            Said::new(format!("{KEYS}/{key}")),
            |said, (field, value)| said.with(field, value.clone()),
        );
        self.human.say(&said).unwrap_or_else(|| {
            tracing::warn!(target: crate::TARGET, key = %said.key, "human text missing");
            format!("{} {:?}", said.key, said.fields)
        })
    }
}
