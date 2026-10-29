//! 桥的工具（施工 O-26，`onebot.md` 第一条「提供者和不说话」，「场所规则和出厂数据」第 1、2 条）。施工 T-1 到 T-4（2026-10-20 项目主人
//! 定：不同平台的工具最终调的接口可能完全不一样，平台的工具在驱动里，im 不提供工具）起都是驱动交的那一套（`crate::toolset::ToolSet`）：
//! 说明照包文件夹的 `data/<目录>/tools/<名字>.json` 读（`miyu_client::tooltext::load::spec_in`，`providers.md`「给模型看的字」），答的话照
//! `data/<目录>/tool-results/` 读，给人看的名字照 `data/human/<语言>.json` 的 `tools.<名字>`（施工 S-6 上，`names`）。im 自己只剩不认识的
//! 工具答的那一句（`data/tool-results/unknown.txt`）。都是给模型看的字，登记在 `docs/ledger.md`；桥起来时和别的出厂数据一起读，写坏了是
//! 打包的错，起不来。
//!
//! 交出去的：`provide` 的参数（[`Tools::provided`]），读的一头当场答的 `tool.call` 的结果（[`Tools::call`]：「这一轮不说话」那一件、
//! 不认识的），交给跟核心的那一头答的（[`Tools::routed`]、[`Tools::driven`]）。
//!
//! 挂几个驱动的（施工 I-70 中上）：每个驱动交一套，合成一份登记（名字照平台前缀分开：`qq_*`、`tg_*`，核心 IM-3 照前缀认平台）。
//! 两套里有同名的是驱动写错了，桥起不来，问题里说是哪一件。每一件记着是哪个平台的那一套：调用照它取平台、配置、状态目录。

use std::path::Path;
use std::sync::Arc;

use miyu_chat::{Problem, Source};
use miyu_client::protocol::Template;
use miyu_client::settings::problem::Code;
use miyu_client::tooltext::Spec;
use miyu_client::tooltext::load::{self, LoadError};
use serde_json::{Map, Value, json};

use super::files;
use super::names::{self, Names};
use crate::chat::Platform;
use crate::toolset::{Answers, ToolSet};

/// 答的那几句在包文件夹的 `data/` 下的哪个目录里（照核心的 `core/tool-results/`）。
const RESULTS: &str = "tool-results";

/// 不认识的工具答的那一句：字段只认 `name`（工具名）。
const UNKNOWN: &str = "unknown";

/// 驱动答「这一轮不说话」的那一句（它的 `ToolSet::answers` 里要有）：不要字段。
const SKIPPED: &str = "skipped";

/// 读好、查过的桥的工具。
#[derive(Debug)]
pub struct Tools {
    /// 不认识的工具答的那一句（`tool-results/unknown.txt`）。
    unknown: Template,
    /// 每件给人看的名字（施工 S-6 上）：照语言，`provide` 时带上（`display_name`）。
    names: Names,
    /// 驱动交的几套（施工 T-1；施工 I-70 中上起一个驱动一套），照驱动的先后。
    driven: Vec<Driven>,
}

/// 一件工具是哪一套的（[`Tools::driven`]）：那个驱动的平台、那一套、它答的话（施工 I-83 从 `Owned` 改名：「owner」专指主人）。
pub(crate) type Toolset = (Platform, Arc<dyn ToolSet>, Arc<Answers>);

/// 驱动交的一套工具，说明、答的话读好了。
struct Driven {
    /// 交它的驱动的平台（施工 I-70 中上）。
    platform: Platform,
    set: Arc<dyn ToolSet>,
    /// 每件的规格和给哪种会话，照 `ToolSet::tools` 的先后。
    specs: Vec<(Spec, &'static [&'static str])>,
    answers: Arc<Answers>,
    /// 驱动给场所会话记事实的模板（施工 Q-3，`ToolSet::facts`）。
    facts: Arc<Answers>,
    /// 驱动问模型的提示词模板（施工 Q-7，`ToolSet::prompts`）。
    prompts: Arc<Answers>,
}

impl std::fmt::Debug for Driven {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Driven")
            .field("platform", &self.platform)
            .field("dir", &self.set.dir())
            .field("specs", &self.specs)
            .finish_non_exhaustive()
    }
}

impl Tools {
    /// `provide` 的参数（「提供者和不说话」第 1 条）：驱动交的每件 `{name, description, input_schema, access, venues}`，有给人看的
    /// 名字的再带 `display_name`（`{语言: 名字}`，施工 S-6 上），写了名字后面跟哪一个参数的再带 `subject`；不写 `timeout_ms`（照核心的
    /// 一分钟）。几套照驱动的先后合成一份（施工 I-70 中上）。每个驱动这一刻的配置在 `configs`（照平台，没有的当空的）：驱动说不登记
    /// 的那几件不带（施工 Q-1，`ToolSet::offers`）。没有驱动的套的是空的。
    pub(crate) fn provided(&self, configs: &[(Platform, Map<String, Value>)]) -> Value {
        let empty = Map::new();
        let tools: Vec<Value> = self
            .driven
            .iter()
            .flat_map(|driven| {
                let config = configs
                    .iter()
                    .find(|(platform, _)| *platform == driven.platform)
                    .map_or(&empty, |(_, config)| config);
                driven
                    .specs
                    .iter()
                    .filter(move |(spec, _)| driven.set.offers(&spec.name, config))
                    .map(move |one| (driven, one))
            })
            .map(|(driven, (spec, venues))| {
                let mut tool = json!({
                    "name": spec.name,
                    "description": spec.description,
                    "input_schema": spec.parameters,
                    "access": spec.access,
                    "venues": venues,
                });
                // 场所插件的工具（施工 S-1 上c，核心 O-60）：只给开着它的场所。
                if let Some(plugin) = driven.set.plugin(&spec.name) {
                    tool["plugin"] = json!(plugin);
                }
                if let Some(face) = self.names.get(&spec.name) {
                    tool["display_name"] = json!(face.names);
                    if let Some(subject) = &face.subject {
                        tool["subject"] = json!(subject);
                    }
                }
                tool
            })
            .collect();
        json!({"tools": tools})
    }

    /// 平台 `platform` 的驱动给场所会话记事实的模板（施工 Q-3）、问模型的提示词模板（施工 Q-7）；那个驱动没交工具的是空的。
    pub(crate) fn texts(&self, platform: Platform) -> Option<(Arc<Answers>, Arc<Answers>)> {
        self.driven
            .iter()
            .find(|driven| driven.platform == platform)
            .map(|driven| (Arc::clone(&driven.facts), Arc::clone(&driven.prompts)))
    }

    /// 「这一轮不说话」那件工具叫什么（施工 T-4，`ToolSet::silence`）：每个驱动的那一件，照驱动的先后（施工 I-70 中上）。
    pub(crate) fn silence(&self) -> Vec<&'static str> {
        self.driven
            .iter()
            .filter_map(|driven| driven.set.silence())
            .collect()
    }

    /// 工具 `tool` 是不是交给跟核心的那一头答的：驱动交的，「这一轮不说话」那一件除外（读的一头当场答）。
    pub(crate) fn routed(&self, tool: &str) -> bool {
        self.silent(tool).is_none() && self.driven(tool).is_some()
    }

    /// 工具 `tool` 是驱动交的（施工 T-1）：交回它是哪个平台的那一套、答的话（施工 I-70 中上）；别的是空的。
    pub(crate) fn driven(&self, tool: &str) -> Option<Toolset> {
        self.driven
            .iter()
            .find(|driven| driven.set.tools().iter().any(|decl| decl.name == tool))
            .map(|driven| {
                (
                    driven.platform,
                    Arc::clone(&driven.set),
                    Arc::clone(&driven.answers),
                )
            })
    }

    /// 工具 `tool` 是哪个驱动的「这一轮不说话」那一件：交回那一套；不是的是空的。
    fn silent(&self, tool: &str) -> Option<&Driven> {
        self.driven
            .iter()
            .find(|driven| driven.set.silence() == Some(tool))
    }

    /// 读的一头当场答的 `tool.call` 的结果（「提供者和不说话」第 2、3 条）：「这一轮不说话」那一件答驱动的那一句，不算出错，带
    /// `end_turn: true`（施工 O-38：核心这一步齐了不再请求，这一轮里又叫她的走新的一轮）；不认识的答 `unknown.txt` 填上那个名字，算出错。
    pub(crate) fn call(&self, tool: &str) -> Value {
        if let Some(driven) = self.silent(tool) {
            let mut answer = driven.answers.say(SKIPPED, &[], false);
            answer["end_turn"] = Value::Bool(true);
            return answer;
        }
        // 起来时试换过 `name`：换得出。
        json!({"blocks": [{"type": "text", "text": load::say(&self.unknown, &[("name", tool)])}], "error": true})
    }
}

/// 读包文件夹的数据目录 `data` 里桥的工具：不认识的那一句、给人看的名字（施工 S-6 上，`names`），驱动交的几套 `toolsets`（每套带
/// 交它的驱动的平台，施工 I-70 中上）各照它的目录读（施工 T-1）。读不成、写坏了、要了别的字段的，每一份记一条 `bad_format`（文件照
/// `data/` 下的位置写，原话照 `miyu_client::tooltext::load` 说的），交回空的；两套里有同名的工具也记（[`duplicates`]，`Factory::load` 照问题整份不收）。
pub(super) fn load(
    data: &Path,
    toolsets: Vec<(Platform, Arc<dyn ToolSet>)>,
    problems: &mut Vec<Problem>,
) -> Option<Tools> {
    duplicates(&toolsets, problems);
    // 给人看的名字先读（施工 S-6 上）：读不懂的那一份先记一条问题，下面记问题的那一个要一直借着 `problems`。
    let named: Vec<&str> = toolsets
        .iter()
        .flat_map(|(_, set)| set.tools().iter().map(|decl| decl.name))
        .collect();
    let names = names::load(data, &named, problems);
    let mut trouble = |file: String, error: LoadError| {
        problems.push(files::whole(
            Code::BadFormat,
            Source::Factory,
            &file,
            Some(error.why),
        ));
    };
    let unknown = load::text_in(data, RESULTS, UNKNOWN, &["name"]);
    let driven: Vec<Option<Driven>> = toolsets
        .into_iter()
        .map(|(platform, set)| driven(data, (platform, set), &mut trouble))
        .collect();
    let driven: Vec<Driven> = driven.into_iter().collect::<Option<_>>()?;
    match unknown {
        Ok(unknown) => Some(Tools {
            unknown,
            names,
            driven,
        }),
        Err(error) => {
            trouble(format!("{RESULTS}/{UNKNOWN}.txt"), error);
            None
        }
    }
}

/// 读驱动交的那一套 `set`（施工 T-1）：说明照 `data/<目录>/tools/`，答的话照 `data/<目录>/tool-results/`。读不成的每一份交给
/// `trouble` 记一条，交回空的。
fn driven(
    data: &Path,
    (platform, set): (Platform, Arc<dyn ToolSet>),
    trouble: &mut impl FnMut(String, LoadError),
) -> Option<Driven> {
    let dir = set.dir();
    let mut specs = Vec::new();
    let mut whole = true;
    for decl in set.tools() {
        match load::spec_in(&data.join(dir), decl.name, decl.access.clone()) {
            Ok(spec) => specs.push((spec, decl.venues)),
            Err(error) => {
                whole = false;
                trouble(format!("{dir}/tools/{}.json", decl.name), error);
            }
        }
    }
    let mut loaded = |read: Result<Answers, Vec<(String, String)>>| match read {
        Ok(answers) => Some(answers),
        Err(broken) => {
            for (file, why) in broken {
                let error = LoadError {
                    file: data.join(&file),
                    why,
                };
                trouble(file, error);
            }
            None
        }
    };
    let answers = loaded(Answers::load(data, dir, set.answers()));
    let facts = loaded(Answers::load_in(data, &format!("{dir}/facts"), set.facts()));
    let prompts = loaded(Answers::load_in(
        data,
        &format!("{dir}/prompts"),
        set.prompts(),
    ));
    let (answers, facts, prompts) = (answers.filter(|_| whole)?, facts?, prompts?);
    Some(Driven {
        platform,
        set,
        specs,
        answers: Arc::new(answers),
        facts: Arc::new(facts),
        prompts: Arc::new(prompts),
    })
}

/// 几套工具 `toolsets` 里同名的（施工 I-70 中上：合成一份登记，名字撞了核心分不出是哪一套的，是驱动写错了）：后出现的那一件每件
/// 记一条 `bad_format`，文件照它那一套的说明写，原话说和哪个平台的撞了。
fn duplicates(toolsets: &[(Platform, Arc<dyn ToolSet>)], problems: &mut Vec<Problem>) {
    let mut seen: Vec<(&str, Platform)> = Vec::new();
    for (platform, set) in toolsets {
        for decl in set.tools() {
            // 名字照平台开头（施工 I-83）：核心照名字的前缀 `<平台>_` 只给那个平台的场所（核心 IM-3），开头不对的会放错地方。
            let head = format!("{}_", platform.name());
            if !decl.name.starts_with(&head) {
                let why = format!(
                    "tool {} of the {} driver does not start with {head}",
                    decl.name,
                    platform.name()
                );
                let file = format!("{}/tools/{}.json", set.dir(), decl.name);
                problems.push(files::whole(
                    Code::BadFormat,
                    Source::Factory,
                    &file,
                    Some(why),
                ));
            }
            match seen.iter().find(|(name, _)| *name == decl.name) {
                Some((_, first)) => {
                    let why = format!(
                        "tool {} is given by the {} driver and the {} driver",
                        decl.name,
                        first.name(),
                        platform.name()
                    );
                    let file = format!("{}/tools/{}.json", set.dir(), decl.name);
                    problems.push(files::whole(
                        Code::BadFormat,
                        Source::Factory,
                        &file,
                        Some(why),
                    ));
                }
                None => seen.push((decl.name, *platform)),
            }
        }
    }
}
