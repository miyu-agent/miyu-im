//! 账本（设计 03 第二节第 4 条）：包的状态目录里一份 JSON，一笔一条；量小，不上数据库。改了整份写到旁边的临时文件、再换过去
//! （原子写回），写到一半断电也不会留下半份。读不进来（坏了）的不覆盖：交回错，记账的那一次说没记上。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 账本的格式版本：改格式的时候往上加，照它迁移。
const VERSION: u32 = 1;

/// 一笔（设计 03 第一节第 1 条）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// 编号：从 1 起，删了的不再用。
    pub id: u64,
    /// 平台（`qq`、`tg`）。
    pub platform: String,
    /// 平台身份（`qq:123`）；只有昵称、没有号的赞助人是空的（10-24 补定第 1 条）。
    pub user: Option<String>,
    /// 记账时的名字；有号又不知道名字的是空的。
    pub name: Option<String>,
    /// 金额（原币种）。
    pub amount: f64,
    /// 币种（三个大写字母）。
    pub currency: String,
    /// 换成哪个币种（记账时的默认币种）。
    pub base: String,
    /// 换成 `base` 的金额；没换算的（汇率查不到）是空的，后台补。
    pub converted: Option<f64>,
    /// 用的汇率：1 `currency` 合多少 `base`（冻结，以后不变）；同币种的是 1。
    pub rate: Option<f64>,
    /// 什么时候记的（毫秒的 Unix 时间）。
    pub at: i64,
    /// 备注。
    pub note: Option<String>,
    /// 谁让记的（平台身份）。
    pub by: Option<String>,
    /// 在哪个场所记的（`qq:group:777`）。
    pub venue: Option<String>,
}

/// 记一笔要的（编号由账本给），各格同 [`Entry`]。
#[derive(Debug, Clone, PartialEq)]
pub struct New {
    /// 同 [`Entry::platform`]。
    pub platform: String,
    /// 同 [`Entry::user`]。
    pub user: Option<String>,
    /// 同 [`Entry::name`]。
    pub name: Option<String>,
    /// 同 [`Entry::amount`]。
    pub amount: f64,
    /// 同 [`Entry::currency`]。
    pub currency: String,
    /// 同 [`Entry::base`]。
    pub base: String,
    /// 同 [`Entry::converted`]。
    pub converted: Option<f64>,
    /// 同 [`Entry::rate`]。
    pub rate: Option<f64>,
    /// 同 [`Entry::at`]。
    pub at: i64,
    /// 同 [`Entry::note`]。
    pub note: Option<String>,
    /// 同 [`Entry::by`]。
    pub by: Option<String>,
    /// 同 [`Entry::venue`]。
    pub venue: Option<String>,
}

/// 文件里的样子。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct File {
    version: u32,
    /// 下一笔的编号。
    next: u64,
    entries: Vec<Entry>,
}

/// 账本：文件和读进来的那一份。
#[derive(Debug)]
pub struct Book {
    path: PathBuf,
    file: File,
}

impl Book {
    /// 读 `path` 的账本；没有的是空的。
    ///
    /// # Errors
    ///
    /// 读不了、不是认得的格式：原话。
    pub fn open(path: &Path) -> Result<Book, String> {
        let file = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice::<File>(&bytes)
                .map_err(|error| format!("{}: {error}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => File {
                version: VERSION,
                next: 1,
                entries: Vec::new(),
            },
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        if file.version > VERSION {
            return Err(format!(
                "{}: version {} is newer than this program",
                path.display(),
                file.version
            ));
        }
        Ok(Book {
            path: path.to_path_buf(),
            file,
        })
    }

    /// 全部，先记的在前。
    pub fn entries(&self) -> &[Entry] {
        &self.file.entries
    }

    /// 记一笔、写回：交回记下的那一笔。
    ///
    /// # Errors
    ///
    /// 写不进去：原话；这时这一笔不算记上。
    pub fn add(&mut self, new: New) -> Result<Entry, String> {
        let entry = Entry {
            id: self.file.next.max(1),
            platform: new.platform,
            user: new.user,
            name: new.name,
            amount: new.amount,
            currency: new.currency,
            base: new.base,
            converted: new.converted,
            rate: new.rate,
            at: new.at,
            note: new.note,
            by: new.by,
            venue: new.venue,
        };
        let mut file = self.file.clone();
        file.next = entry.id + 1;
        file.version = VERSION;
        file.entries.push(entry.clone());
        write(&self.path, &file)?;
        self.file = file;
        Ok(entry)
    }

    /// 改编号是 `id` 的那一笔（`change` 改它）、写回：交回改过的那一笔；没有这一笔的是空的。
    ///
    /// # Errors
    ///
    /// 写不进去：原话；这时不算改了。
    pub fn update(
        &mut self,
        id: u64,
        change: impl FnOnce(&mut Entry),
    ) -> Result<Option<Entry>, String> {
        let mut file = self.file.clone();
        let Some(entry) = file.entries.iter_mut().find(|entry| entry.id == id) else {
            return Ok(None);
        };
        change(entry);
        entry.id = id;
        let changed = entry.clone();
        write(&self.path, &file)?;
        self.file = file;
        Ok(Some(changed))
    }

    /// 删编号是 `id` 的那一笔、写回：交回删没删（没有这一笔的是假的）。编号不再用。
    ///
    /// # Errors
    ///
    /// 写不进去：原话；这时不算删了。
    pub fn delete(&mut self, id: u64) -> Result<bool, String> {
        let mut file = self.file.clone();
        let before = file.entries.len();
        file.entries.retain(|entry| entry.id != id);
        if file.entries.len() == before {
            return Ok(false);
        }
        write(&self.path, &file)?;
        self.file = file;
        Ok(true)
    }
}

/// 整份写到旁边的临时文件、再换过去。
fn write(path: &Path, file: &File) -> Result<(), String> {
    let text = serde_json::to_vec_pretty(file).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text)
        .map_err(|error| format!("{}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, path).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
