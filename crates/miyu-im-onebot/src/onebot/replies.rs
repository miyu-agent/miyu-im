//! 读 NapCat 的回应（施工 I-2 上）：回应的 `data` 读成宿主认的样子（[`miyu_im::chat`] 的 [`Fetched`]、[`Member`] 这些）。只读、
//! 不洗：名字合不合核心的写法由宿主洗。号照 [`number`]：整数，或者写成整数的字符串。

use serde_json::Value;

use super::media::{file_name, sources};
use super::{display_name, number, rank_of, segments};
use miyu_im::chat::{Fetched, Friend, Group, GroupFile, GroupFiles, Located, Member};

/// 动作的名字：群列表、好友列表、群成员列表、群资料（施工 O-58 上、O-54、O-32）。
pub const GROUP_LIST: &str = "get_group_list";
/// 同 [`GROUP_LIST`]。
pub const FRIEND_LIST: &str = "get_friend_list";
/// 同 [`GROUP_LIST`]。
pub const MEMBER_LIST: &str = "get_group_member_list";
/// 同 [`GROUP_LIST`]。
pub const GROUP_INFO: &str = "get_group_info";
/// 同 [`GROUP_LIST`]：群文件最外一层、一个文件夹里的、一个文件的下载地址（施工 I-12）。
pub const ROOT_FILES: &str = "get_group_root_files";
/// 同 [`ROOT_FILES`]。
pub const FOLDER_FILES: &str = "get_group_files_by_folder";
/// 同 [`ROOT_FILES`]。
pub const FILE_URL: &str = "get_group_file_url";
/// 同 [`GROUP_LIST`]：机器人自己的号和昵称（施工 I-6 中）。
pub const LOGIN_INFO: &str = "get_login_info";

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

/// `get_group_root_files`、`get_group_files_by_folder` 回应的 `data`（施工 I-12）：`folders` 里的 `folder_id`、`folder_name`、
/// `total_file_count`，`files` 里的 `file_id`、`file_name`、`size`（或 `file_size`）、`uploader_name`；编号、名字读不出的不要。
pub fn group_files(data: &Value) -> GroupFiles {
    let text = |value: &Value| {
        value
            .as_str()
            .filter(|text| !text.is_empty())
            .map(str::to_string)
    };
    GroupFiles {
        folders: rows(&data["folders"])
            .filter_map(|row| {
                Some((
                    text(&row["folder_id"])?,
                    text(&row["folder_name"])?,
                    row["total_file_count"].as_u64().unwrap_or_default(),
                ))
            })
            .collect(),
        files: rows(&data["files"])
            .filter_map(|row| {
                Some(GroupFile {
                    id: text(&row["file_id"])?,
                    name: text(&row["file_name"])?,
                    size: row["size"]
                        .as_u64()
                        .or_else(|| row["file_size"].as_u64())
                        .unwrap_or_default(),
                    uploader: text(&row["uploader_name"]),
                })
            })
            .collect(),
    }
}

/// `get_group_file_url` 回应的 `data` 里的 `url`；没有的、空的是空的（施工 I-12）。
pub fn file_url(data: &Value) -> Option<String> {
    data["url"]
        .as_str()
        .filter(|url| !url.is_empty())
        .map(str::to_string)
}

/// `get_login_info` 回应的 `data` 里的昵称 `nickname`，照原样；不是字的、空白的是空的（施工 I-6 中）。
pub fn nickname(data: &Value) -> Option<String> {
    data["nickname"]
        .as_str()
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
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
