//! 桥的工具（施工 O-26，`onebot.md` 第一条「提供者和不说话」，「场所规则和出厂数据」第 1、2 条）。施工 T-1 到 T-4（2026-10-20 项目主人
//! 定：不同平台的工具最终调的接口可能完全不一样，平台的工具在驱动里，im 不提供工具）起都是驱动交的那一套（`crate::toolset::ToolSet`）：
//! 说明照包文件夹的 `data/<目录>/tools/<名字>.json` 读（`miyu_client::tooltext::load::spec_in`，`providers.md`「给模型看的字」），答的话照
//! `data/<目录>/tool-results/` 读，给人看的名字照 `data/human/<语言>.json` 的 `tools.<名字>`（施工 S-6 上，`names`）。im 自己只剩不认识的
//! 工具答的那一句（`data/tool-results/unknown.txt`）。都是给模型看的字，登记在 `docs/ledger.md`；桥起来时和别的出厂数据一起读，写坏了是
//! 打包的错，起不来。
//!
//! 交出去的：`provide` 的参数（[`Tools::provided`]），读的一头当场答的 `tool.call` 的结果（[`Tools::call`]：「这一轮不说话」那一件、
//! 不认识的），交给跟核心的那一头答的（[`Tools::routed`]、[`Tools::driven`]）。

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
    /// 驱动交的那一套（施工 T-1）。
    driven: Option<Driven>,
}

/// 驱动交的一套工具，说明、答的话读好了。
struct Driven {
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
            .field("dir", &self.set.dir())
            .field("specs", &self.specs)
            .finish_non_exhaustive()
    }
}

impl Tools {
    /// `provide` 的参数（「提供者和不说话」第 1 条）：驱动交的每件 `{name, description, input_schema, access, venues}`，有给人看的
    /// 名字的再带 `display_name`（`{语言: 名字}`，施工 S-6 上），写了名字后面跟哪一个参数的再带 `subject`；不写 `timeout_ms`（照核心的
    /// 一分钟）。驱动的配置是 `config`：驱动说不登记的那几件不带（施工 Q-1，`ToolSet::offers`）。没有驱动的套的是空的。
    pub(crate) fn provided(&self, config: &Map<String, Value>) -> Value {
        let tools: Vec<Value> = self
            .driven
            .iter()
            .flat_map(|driven| {
                driven
                    .specs
                    .iter()
                    .filter(|(spec, _)| driven.set.offers(&spec.name, config))
            })
            .map(|(spec, venues)| {
                let mut tool = json!({
                    "name": spec.name,
                    "description": spec.description,
                    "input_schema": spec.parameters,
                    "access": spec.access,
                    "venues": venues,
                });
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

    /// 驱动给场所会话记事实的模板（施工 Q-3）、问模型的提示词模板（施工 Q-7）；没有驱动的是空的。
    pub(crate) fn texts(&self) -> Option<(Arc<Answers>, Arc<Answers>)> {
        self.driven
            .as_ref()
            .map(|driven| (Arc::clone(&driven.facts), Arc::clone(&driven.prompts)))
    }

    /// 「这一轮不说话」那件工具叫什么（施工 T-4，`ToolSet::silence`）。
    pub(crate) fn silence(&self) -> Option<&'static str> {
        self.driven.as_ref()?.set.silence()
    }

    /// 工具 `tool` 是不是交给跟核心的那一头答的：驱动交的，「这一轮不说话」那一件除外（读的一头当场答）。
    pub(crate) fn routed(&self, tool: &str) -> bool {
        self.silence() != Some(tool) && self.driven(tool).is_some()
    }

    /// 工具 `tool` 是驱动交的（施工 T-1）：交回那一套和它答的话；别的是空的。
    pub(crate) fn driven(&self, tool: &str) -> Option<(Arc<dyn ToolSet>, Arc<Answers>)> {
        let driven = self.driven.as_ref()?;
        driven
            .set
            .tools()
            .iter()
            .any(|decl| decl.name == tool)
            .then(|| (Arc::clone(&driven.set), Arc::clone(&driven.answers)))
    }

    /// 读的一头当场答的 `tool.call` 的结果（「提供者和不说话」第 2、3 条）：「这一轮不说话」那一件答驱动的那一句，不算出错，带
    /// `end_turn: true`（施工 O-38：核心这一步齐了不再请求，这一轮里又叫她的走新的一轮）；不认识的答 `unknown.txt` 填上那个名字，算出错。
    pub(crate) fn call(&self, tool: &str) -> Value {
        if let Some(driven) = self
            .driven
            .as_ref()
            .filter(|_| self.silence() == Some(tool))
        {
            let mut answer = driven.answers.say(SKIPPED, &[], false);
            answer["end_turn"] = Value::Bool(true);
            return answer;
        }
        // 起来时试换过 `name`：换得出。
        json!({"blocks": [{"type": "text", "text": load::say(&self.unknown, &[("name", tool)])}], "error": true})
    }
}

/// 读包文件夹的数据目录 `data` 里桥的工具：不认识的那一句、给人看的名字（施工 S-6 上，`names`），驱动交的那一套 `toolset` 照它的
/// 目录读（施工 T-1）。读不成、写坏了、要了别的字段的，每一份记一条 `bad_format`（文件照 `data/` 下的位置写，原话照
/// `miyu_client::tooltext::load` 说的），交回空的。
pub(super) fn load(
    data: &Path,
    toolset: Option<Arc<dyn ToolSet>>,
    problems: &mut Vec<Problem>,
) -> Option<Tools> {
    // 给人看的名字先读（施工 S-6 上）：读不懂的那一份先记一条问题，下面记问题的那一个要一直借着 `problems`。
    let named: Vec<&str> = toolset
        .iter()
        .flat_map(|set| set.tools().iter().map(|decl| decl.name))
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
    let driven = toolset.map(|set| driven(data, set, &mut trouble));
    let driven = match driven {
        None => None,
        Some(Some(driven)) => Some(driven),
        Some(None) => return None,
    };
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
    set: Arc<dyn ToolSet>,
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
        set,
        specs,
        answers: Arc::new(answers),
        facts: Arc::new(facts),
        prompts: Arc::new(prompts),
    })
}
