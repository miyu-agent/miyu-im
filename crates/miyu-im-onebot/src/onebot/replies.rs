//! 读 NapCat 的回应（施工 I-2 上）：回应的 `data` 读成宿主认的样子（[`miyu_im::chat`] 的 [`Fetched`]、[`Member`] 这些）。只读、
//! 不洗：名字合不合核心的写法由宿主洗。号照 [`number`]：整数，或者写成整数的字符串。

use serde_json::Value;

use super::media::{file_name, sources};
use super::{display_name, number, rank_of, segments};
use miyu_im::chat::{Fetched, Friend, Group, Located, Member};

/// 动作的名字：群列表、好友列表、群成员列表、群资料（施工 O-58 上、O-54、O-32）。
pub const GROUP_LIST: &str = "get_group_list";
/// 同 [`GROUP_LIST`]。
pub const FRIEND_LIST: &str = "get_friend_list";
/// 同 [`GROUP_LIST`]。
pub const MEMBER_LIST: &str = "get_group_member_list";
/// 同 [`GROUP_LIST`]。
pub const GROUP_INFO: &str = "get_group_info";

/// `get_msg` 回应的 `data`：`message_type` 是 `private` 的是私聊里的；群号 `group_id`、发的人 `user_id`；段照 `message` 认。
pub fn fetched(data: &Value) -> Fetched {
    Fetched {
        private: data["message_type"] == "private",
        group: number(&data["group_id"]),
        user: number(&data["user_id"]),
        segments: segments(&data["message"]),
    }
}

/// `get_image`、`get_file` 回应的 `data`：东西在哪（[`sources`]）、文件名（[`file_name`]）。
pub fn located(data: &Value) -> Located {
    Located {
        sources: sources(data),
        name: file_name(data).map(str::to_string),
    }
}

/// `get_group_member_info` 回应的 `data`、`get_group_member_list` 的一行：号读不出的不是。
pub fn member(data: &Value) -> Option<Member> {
    Some(Member {
        user: number(&data["user_id"])?,
        name: display_name(data),
        rank: rank_of(data),
    })
}

/// `get_group_member_list` 回应的 `data`：每一行照 [`member`]，号读不出的不要。
pub fn members(data: &Value) -> Vec<Member> {
    rows(data).filter_map(member).collect()
}

/// `get_group_list` 回应的 `data`：群号 `group_id`、群名 `group_name`、人数 `member_count`；群号读不出的不要。
pub fn groups(data: &Value) -> Vec<Group> {
    rows(data)
        .filter_map(|row| {
            Some(Group {
                id: number(&row["group_id"])?,
                name: row["group_name"].as_str().map(str::to_string),
                members: row["member_count"].as_u64(),
            })
        })
        .collect()
}

/// `get_friend_list` 回应的 `data`：号 `user_id`、昵称 `nickname`；号读不出的不要。
pub fn friends(data: &Value) -> Vec<Friend> {
    rows(data)
        .filter_map(|row| {
            Some(Friend {
                id: number(&row["user_id"])?,
                name: row["nickname"].as_str().map(str::to_string),
            })
        })
        .collect()
}

/// `get_group_info` 回应的 `data` 里的群名 `group_name`，照原样；不是字的是空的。
pub fn group_name(data: &Value) -> Option<String> {
    data["group_name"].as_str().map(str::to_string)
}

/// 一张表的每一行：不是数组的什么都没有。
fn rows(data: &Value) -> impl Iterator<Item = &Value> {
    data.as_array().into_iter().flatten()
}

#[cfg(test)]
mod tests;
