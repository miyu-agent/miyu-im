//! 一个机器人收消息时记下的（施工 I-183 中下）：发消息要用、Bot API 又不好问的几样，收的时候顺手记着。
//!
//! - 每个聊天最近一条消息在哪个话题（`message_thread_id`）：话题群整个群一个场所，回话回在原来那个话题里（设计 07 第 3.5 节）。
//! - 人的名字和用户名：@ 一个人时，有用户名的写 `@用户名`（Telegram 自己认），没有的写名字、挂一个 `text_mention`。
//! - 群名：宿主问群名（`Link::group_name`）时照它答。
//!
//! 只在内存里，重启从头记；最多记 [`LIMIT`] 个，满了清掉重记（只是少几个名字，不会错）。

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

/// 每一样最多记多少个。
const LIMIT: usize = 4096;

/// 一个人：名字、用户名（没有的空）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    /// 名加姓。
    pub name: String,
    /// 用户名，不带 `@`；没有的是空的。
    pub username: Option<String>,
}

/// 记着的几样。
#[derive(Debug, Default)]
pub struct Known {
    threads: Mutex<HashMap<String, i64>>,
    people: Mutex<HashMap<String, Person>>,
    titles: Mutex<HashMap<String, String>>,
    /// 命令菜单里登记的名字到 Miyu 的命令名（施工 I-183 下上：Telegram 不许 `-`，登记时换成 `_`，收到时换回来）。
    commands: Mutex<HashMap<String, String>>,
    /// 群：是不是话题群、机器人是不是管理员（没问到的不在里面；施工 I-183 下中，会话浮窗的「隐私模式」「话题群」标签）。
    groups: Mutex<HashMap<String, Group>>,
}

/// 一个群记下的（[`Known::group`]）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Group {
    /// 话题群（消息里 `chat.is_forum`）。
    pub forum: bool,
    /// 机器人是管理员；还没问到的是空的。
    pub admin: Option<bool>,
    /// 问过了（问着的、问到的、问不成的）：不再问。
    asked: bool,
}

/// 往 `map` 里记一样：满了先清掉。
fn remember<V>(map: &Mutex<HashMap<String, V>>, key: &str, value: V) {
    let mut held = map.lock().unwrap_or_else(PoisonError::into_inner);
    if held.len() >= LIMIT && !held.contains_key(key) {
        held.clear();
    }
    held.insert(key.to_string(), value);
}

impl Known {
    /// 聊天 `chat` 最近一条在话题 `thread` 里；不在话题里的（普通群、话题群的「General」）是空的，拿掉记着的。
    pub fn set_thread(&self, chat: &str, thread: Option<i64>) {
        match thread {
            Some(thread) => remember(&self.threads, chat, thread),
            None => {
                self.threads
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(chat);
            }
        }
    }

    /// 聊天 `chat` 回话回在哪个话题；没有的是空的。
    pub fn thread(&self, chat: &str) -> Option<i64> {
        self.threads
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(chat)
            .copied()
    }

    /// 记下号是 `user` 的人。
    pub fn set_person(&self, user: &str, person: Person) {
        remember(&self.people, user, person);
    }

    /// 号是 `user` 的人；没见过的是空的。
    pub fn person(&self, user: &str) -> Option<Person> {
        self.people
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(user)
            .cloned()
    }

    /// 记下群 `chat` 叫什么。
    pub fn set_title(&self, chat: &str, title: &str) {
        remember(&self.titles, chat, title.to_string());
    }

    /// 换上命令菜单的对照：登记的名字 → Miyu 的命令名。
    pub fn set_commands(&self, commands: HashMap<String, String>) {
        *self.commands.lock().unwrap_or_else(PoisonError::into_inner) = commands;
    }

    /// 命令菜单里登记的名字 `name` 对应的 Miyu 命令名；不是菜单里的是空的。
    pub fn command(&self, name: &str) -> Option<String> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(name)
            .cloned()
    }

    /// 收到群 `chat` 的一条：记下是不是话题群；交回这个群的机器人是不是管理员还没问过（要问一次）。
    pub fn seen_group(&self, chat: &str, forum: bool) -> bool {
        let mut held = self.groups.lock().unwrap_or_else(PoisonError::into_inner);
        if held.len() >= LIMIT && !held.contains_key(chat) {
            held.clear();
        }
        let group = held.entry(chat.to_string()).or_default();
        group.forum = forum;
        let ask = !group.asked;
        group.asked = true;
        ask
    }

    /// 记下群 `chat` 里机器人是不是管理员。
    pub fn set_admin(&self, chat: &str, admin: bool) {
        if let Some(group) = self
            .groups
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(chat)
        {
            group.admin = Some(admin);
        }
    }

    /// 群 `chat` 记下的；没收到过消息的是空的。
    pub fn group(&self, chat: &str) -> Option<Group> {
        self.groups
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(chat)
            .copied()
    }

    /// 群 `chat` 叫什么；没见过的是空的。
    pub fn title(&self, chat: &str) -> Option<String> {
        self.titles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(chat)
            .cloned()
    }
}

#[cfg(test)]
mod tests;
