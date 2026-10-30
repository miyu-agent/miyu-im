//! 说给人听的字（施工 O-8，`onebot.md` 第一条「给人看的字」）：字在 `miyu-im/data/human/`，三种语言里桥说的
//! 每一句都换得出来（换不出来的会印出说法的编号），字段换进去；中文照图纸；日文照英文；换语言照新的说；握手以前照系统的
//! 语言（施工 O-20：不读配置）。O-18 多了 `start`、`stop`、`restart`、`status`、`logs` 说的；O-21 多了 `venue show` 说的、
//! 出厂的数据有问题、问题说成话；O-23 多了限流满了发进群里的那一句；O-28 下去掉桥自己的网页说的几句，`status` 多了设置在哪
//! 那一句；O-28 补多了 `web` 找不到 `miyu` 那一句，设置在哪改说 `miyu im web`。

use miyu_chat::{Entry, Origin, Problem, Source};
use miyu_config::Value;
use miyu_config::problem::{At, Code};
use miyu_im::chat::Id;
use miyu_im::control::{Halt, Report};
use miyu_im::logs::Heading;
use miyu_im::serve::{Failure, Notice};
use miyu_im::texts::{Texts, system_language};
use miyu_im::venue::Shown;
use miyu_store::resources::ResourceRoot;

/// 出厂的资源目录（拼成的，施工 S-6 中）。
fn resources() -> ResourceRoot {
    ResourceRoot::at(crate::support::resources())
}

/// 一条问题：系统的 `80-x.toml` 第 1 条规则第 3 行的 `rate`，原文 `"abc"`，原因码是 `code`。
fn problem(code: Code) -> Problem {
    Problem {
        code,
        source: Source::System,
        file: "80-x.toml".to_string(),
        rule: Some(1),
        key: Some("rate".to_string()),
        at: Some(At { line: 3, column: 8 }),
        got: Some("\"abc\"".to_string()),
        suggest: None,
        why: None,
    }
}

/// 每一种问题：每一种原因码，在哪的三种（有第几条规则、只有第几行、整份文件），不认识的键有没有最近的名字，缺了的。
fn problems() -> Vec<Problem> {
    let mut all: Vec<Problem> = [
        Code::Unreadable,
        Code::TooBig,
        Code::NotUtf8,
        Code::Syntax,
        Code::UnknownKey,
        Code::WrongType,
        Code::NotAnOption,
        Code::OutOfRange,
        Code::BadFormat,
        Code::UnknownSecret,
    ]
    .into_iter()
    .map(problem)
    .collect();
    all.push(Problem {
        suggest: Some("rate".to_string()),
        key: Some("rat".to_string()),
        ..problem(Code::UnknownKey)
    });
    all.push(Problem {
        source: Source::Factory,
        file: "defaults.toml".to_string(),
        rule: None,
        ..problem(Code::OutOfRange)
    });
    all.push(Problem {
        source: Source::Factory,
        file: "defaults.toml".to_string(),
        rule: None,
        at: None,
        got: None,
        key: Some("chatty.base".to_string()),
        ..problem(Code::WrongType)
    });
    all.push(Problem {
        rule: None,
        key: None,
        at: None,
        got: None,
        why: Some("permission denied".to_string()),
        ..problem(Code::Unreadable)
    });
    all
}

/// 桥说的每一句：用法、日志、起来连上断开、起不来停了、开关和状态、`logs` 的标题、`venue show` 的
/// 每一句和每一种问题。
fn everything(texts: &Texts) -> Vec<String> {
    let mut said = vec![
        texts.help(None).unwrap_or_default(),
        // 每个命令的帮助页（施工 I-87）。
        [
            "start",
            "stop",
            "restart",
            "status",
            "web",
            "logs",
            "venue",
            "whitelist",
            "access",
        ]
        .iter()
        .filter_map(|command| texts.help(Some(command)))
        .collect::<Vec<_>>()
        .join("\n"),
        texts.no_command(),
        texts.unknown_command("nonsense"),
        texts.bad_args("logs"),
        texts.no_log("disk full"),
        texts.web_not_running(),
        texts.web_failed("refused"),
        texts.web_opened("http://127.0.0.1:8302/", true),
        texts.web_opened("http://127.0.0.1:8302/", false),
    ];
    for notice in [
        Notice::Listening {
            host: "127.0.0.1".to_string(),
            port: 8301,
        },
        Notice::Connected {
            bot: Some(Id::from(30003)),
        },
        Notice::Connected { bot: None },
        Notice::Disconnected {
            bot: Some(Id::from(30003)),
        },
        Notice::Disconnected { bot: None },
        Notice::NoToken,
        Notice::Page { port: 8302 },
        Notice::PageInUse { port: 8302 },
    ] {
        said.push(texts.notice(&notice));
    }
    for failure in [
        Failure::Core("refused".to_string()),
        Failure::NotSpawned,
        Failure::PortInUse(8301),
        Failure::Crashed("boom".to_string()),
        Failure::Start("nope".to_string()),
        Failure::Factory(problems()),
    ] {
        said.push(texts.failure(&failure));
    }
    for shown in [
        Shown::None,
        Shown::Defaults,
        Shown::Problems,
        Shown::BadVenue("qq:room:1".to_string()),
    ] {
        said.push(texts.shown(&shown));
    }
    said.push(texts.entry(
        "rate",
        &Entry {
            value: Value::Text("5/300s".into()),
            origin: Origin {
                source: Source::Factory,
                file: "50-defaults.toml".to_string(),
                rule: 1,
                line: 15,
            },
        },
    ));
    for problem in problems() {
        said.push(texts.problem(&problem));
    }
    for halt in [
        Halt::ConfigError,
        Halt::FailedRepeatedly(5),
        Halt::NotInstalled,
        Halt::CannotStart,
        Halt::ProtocolMismatch,
        Halt::Other("new_reason".to_string()),
    ] {
        said.push(texts.report(&Report::Halted(halt)));
    }
    for report in [
        Report::Started,
        Report::Stopped,
        Report::Restarted,
        Report::Off,
        Report::Starting,
        Report::Running(4242),
        Report::Waiting {
            seconds: 2,
            failures: 1,
        },
        Report::Stderr,
        Report::Other("dreaming".to_string()),
        Report::Missing,
        Report::Napcat {
            implementation: "NapCat.Onebot".to_string(),
            version: "4.8.2".to_string(),
            bot: "30003".to_string(),
        },
        Report::NapcatBot("30003".to_string()),
        Report::NoNapcat,
        Report::Listen(8301),
        Report::Page,
    ] {
        said.push(texts.report(&report));
    }
    for heading in [
        Heading::Stderr("state/logs/miyu-im.stderr".to_string()),
        Heading::Log("state/logs/miyu-im.log".to_string()),
        Heading::None("state/logs/miyu-im.log".to_string()),
    ] {
        said.push(texts.heading(&heading));
    }
    said
}

#[test]
fn every_sentence_turns_into_words_in_every_language() {
    for language in ["zh", "en", "ja"] {
        let texts = Texts::load(resources(), crate::support::package(), language)
            .expect("出厂的字读得出来");
        for said in everything(&texts) {
            assert!(
                !said.contains("software/im/"),
                "{language}：换不出来：{said}"
            );
            assert!(!said.trim().is_empty(), "{language}：空的");
        }
    }
}

#[test]
fn chinese_reads_as_drawn_and_fields_go_in() {
    let texts = Texts::load(resources(), crate::support::package(), "zh").expect("读得出来");
    let listening = Notice::Listening {
        host: "127.0.0.1".to_string(),
        port: 8301,
    };
    assert_eq!(
        texts.notice(&listening),
        "在 127.0.0.1:8301 等待 NapCat 连接"
    );
    assert_eq!(
        texts.notice(&Notice::Disconnected {
            bot: Some(Id::from(30003))
        }),
        "NapCat 已断开（机器人 30003），等待重连"
    );
    // 施工 I-87：报错 `miyu im: …`，建议另起一行「提示：」，结果行不带句号。
    assert_eq!(
        texts.failure(&Failure::Core("refused".to_string())),
        "miyu im: 无法连接核心：refused"
    );
    assert_eq!(
        texts.notice(&Notice::NoToken),
        "miyu im: 警告：没有设置 Token，NapCat 的连接会被拒绝\n提示：用 'miyu im web' 打开管理页面，粘贴 NapCat 的 Token"
    );
    assert!(
        texts
            .help(None)
            .is_some_and(|help| help.starts_with("用法：miyu im <命令>\n"))
    );
    assert_eq!(
        texts.unknown_command("stat"),
        "miyu im: 未知命令 'stat'\n提示：运行 'miyu im --help' 查看可用命令"
    );
    assert_eq!(
        texts.bad_args("logs"),
        "miyu im: 'logs' 的参数不对\n提示：运行 'miyu im logs --help' 查看用法"
    );
    assert_eq!(
        texts.failure(&Failure::NotSpawned),
        "miyu im: 没有收到核心的握手回应\n提示：'miyu-im serve' 只能由核心启动，用 'miyu im start' 启动"
    );
    assert_eq!(texts.report(&Report::Started), "已启动接入通讯平台");
    assert_eq!(texts.report(&Report::Stopped), "已停止接入通讯平台");
    assert_eq!(texts.report(&Report::Restarted), "已重启接入通讯平台");
    assert_eq!(
        texts.report(&Report::Off),
        "接入通讯平台未运行\n提示：用 'miyu im start' 启动"
    );
    assert_eq!(texts.report(&Report::Starting), "接入通讯平台正在启动");
    assert_eq!(
        texts.report(&Report::Running(4242)),
        "接入通讯平台正在运行（进程 4242）"
    );
    assert_eq!(
        texts.report(&Report::Waiting {
            seconds: 2,
            failures: 1
        }),
        "接入通讯平台已退出，2 秒后重新启动（已连续失败 1 次）"
    );
    let halted = |reason: &str| {
        format!(
            "接入通讯平台已停止，不再自动启动：{reason}\n提示：运行 'miyu im logs' 查看日志\n提示：修复后运行 'miyu im restart'"
        )
    };
    assert_eq!(
        texts.report(&Report::Halted(Halt::ConfigError)),
        halted("配置错误或端口被占用（退出码 1）")
    );
    assert_eq!(
        texts.report(&Report::Halted(Halt::FailedRepeatedly(5))),
        halted("连续失败 5 次")
    );
    assert_eq!(
        texts.report(&Report::Halted(Halt::NotInstalled)),
        halted("找不到 miyu-im 程序")
    );
    assert_eq!(
        texts.report(&Report::Halted(Halt::CannotStart)),
        halted("无法启动进程，或无法创建它的目录和标准错误文件")
    );
    assert_eq!(
        texts.report(&Report::Halted(Halt::ProtocolMismatch)),
        halted("清单的协议版本和核心不一致")
    );
    assert_eq!(
        texts.report(&Report::Halted(Halt::Other("new_reason".to_string()))),
        halted("new_reason"),
        "不认识的原因照原样"
    );
    assert_eq!(texts.report(&Report::Stderr), "标准错误的最后几行：");
    assert_eq!(
        texts.report(&Report::Other("dreaming".to_string())),
        "接入通讯平台状态：dreaming"
    );
    assert_eq!(
        texts.report(&Report::Missing),
        "miyu im: 核心里没有 miyu-im 软件包\n提示：用 'miyu check' 检查"
    );
    assert_eq!(
        texts.report(&Report::Napcat {
            implementation: "NapCat.Onebot".to_string(),
            version: "4.8.2".to_string(),
            bot: "30003".to_string(),
        }),
        "NapCat 已连接：NapCat.Onebot 4.8.2，机器人 30003"
    );
    assert_eq!(
        texts.report(&Report::NapcatBot("30003".to_string())),
        "NapCat 已连接：机器人 30003"
    );
    assert_eq!(texts.report(&Report::NoNapcat), "NapCat 未连接");
    assert_eq!(
        texts.report(&Report::Listen(8301)),
        "NapCat 连接地址：ws://127.0.0.1:8301/ws"
    );
    assert_eq!(
        texts.web_opened("http://127.0.0.1:8302/#miyu=x", false),
        "在浏览器中打开以下地址（1 分钟内有效）：http://127.0.0.1:8302/#miyu=x"
    );
    assert_eq!(
        texts.report(&Report::Page),
        "提示：用 'miyu im web' 打开管理页面"
    );
    assert_eq!(
        texts.heading(&Heading::Stderr("a/miyu-im.stderr".to_string())),
        "==> 标准错误：a/miyu-im.stderr <=="
    );
    assert_eq!(
        texts.heading(&Heading::Log("a/miyu-im.log".to_string())),
        "==> 运行日志：a/miyu-im.log <=="
    );
    assert_eq!(
        texts.heading(&Heading::None("a/miyu-im.log".to_string())),
        "miyu im: 还没有运行日志：a/miyu-im.log"
    );
}

#[test]
fn japanese_is_its_own_and_the_language_can_change() {
    // 施工 O-45：日文另写一份（原来照英文写），说的每一句都不和英文一样。
    let english = Texts::load(resources(), crate::support::package(), "en").expect("读得出来");
    let mut texts = Texts::load(resources(), crate::support::package(), "ja").expect("读得出来");
    let same: Vec<String> = everything(&texts)
        .into_iter()
        .zip(everything(&english))
        .filter(|(ja, en)| ja == en)
        .map(|(ja, _)| ja)
        .collect();
    assert_eq!(same, Vec::<String>::new(), "和英文一样的");
    assert!(
        texts
            .help(None)
            .is_some_and(|help| help.starts_with("使い方：miyu im <コマンド>\n"))
    );
    assert_eq!(
        texts.report(&Report::Halted(Halt::ConfigError)),
        "メッセージング接続は停止し、自動では再起動しません：設定の誤りかポートの使用中（終了コード 1）\nヒント：'miyu im logs' でログを表示\nヒント：直したら 'miyu im restart' を実行"
    );
    texts.speak("zh").expect("读得出来");
    assert_eq!(texts.report(&Report::NoNapcat), "NapCat 未连接");
}

#[test]
fn before_the_handshake_the_system_language_is_spoken() {
    assert_eq!(system_language(Some("zh_CN.UTF-8")), "zh");
    assert_eq!(system_language(Some("ja_JP")), "ja");
    assert_eq!(system_language(Some("de_DE")), "en");
    assert_eq!(system_language(None), "en");
}

#[test]
fn venue_show_and_problems_read_as_drawn() {
    let texts = Texts::load(resources(), crate::support::package(), "zh").expect("读得出来");
    let said: Vec<String> = problems().iter().map(|one| texts.problem(one)).collect();
    assert_eq!(
        said,
        [
            "系统 80-x.toml 第 1 条规则，第 3 行：读不了：",
            "系统 80-x.toml 第 1 条规则，第 3 行：超过 1 MiB，不读",
            "系统 80-x.toml 第 1 条规则，第 3 行：不是 UTF-8，不读",
            "系统 80-x.toml 第 1 条规则，第 3 行：TOML 写法不对：",
            "系统 80-x.toml 第 1 条规则，第 3 行：未知的键 rate",
            "系统 80-x.toml 第 1 条规则，第 3 行：rate 的类型不对：\"abc\"",
            "系统 80-x.toml 第 1 条规则，第 3 行：rate 不是能选的值：\"abc\"",
            "系统 80-x.toml 第 1 条规则，第 3 行：rate 超出范围：\"abc\"",
            "系统 80-x.toml 第 1 条规则，第 3 行：rate 写法不对：\"abc\"",
            "系统 80-x.toml 第 1 条规则，第 3 行：rate：unknown_secret",
            "系统 80-x.toml 第 1 条规则，第 3 行：未知的键 rat（也许是 rate）",
            "出厂 defaults.toml 第 3 行：rate 超出范围：\"abc\"",
            "出厂 defaults.toml：缺了 chatty.base",
            "系统 80-x.toml：读不了：permission denied",
        ]
    );
    assert_eq!(
        texts.failure(&Failure::Factory(problems()[..2].to_vec())),
        "miyu im: 出厂数据有问题，无法启动（打包错误）：\n  \
         系统 80-x.toml 第 1 条规则，第 3 行：读不了：\n  \
         系统 80-x.toml 第 1 条规则，第 3 行：超过 1 MiB，不读"
    );
    assert_eq!(texts.shown(&Shown::None), "没有规则设置这个场所");
    assert_eq!(
        texts.shown(&Shown::Defaults),
        "未列出的参数使用出厂的 defaults.toml"
    );
    assert_eq!(
        texts.shown(&Shown::Problems),
        "读取时发现以下问题（有问题的项、规则或文件不生效）："
    );
    assert_eq!(
        texts.shown(&Shown::BadVenue("qq:room:1".to_string())),
        "miyu im: 无法识别场所编号 'qq:room:1'\n提示：写成 '<平台>:<机器人号>:group:<群号>' 或 '<平台>:<机器人号>:private:<号>'，例如 'qq:10001:group:123456'"
    );
    let english = Texts::load(resources(), crate::support::package(), "en").expect("读得出来");
    assert_eq!(
        english.problem(&problems()[12]),
        "factory defaults.toml: chatty.base is missing"
    );
}
