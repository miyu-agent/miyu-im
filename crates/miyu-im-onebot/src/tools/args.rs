//! 参数的读法：挪进了宿主（[`miyu_im::toolset::common::args`]，施工 I-183 下下，Telegram 的工具同用），这里照 QQ 的叫法接着用。

pub(crate) use miyu_im::toolset::common::args::{
    Bad, bad_args, group_only, id as qq_id, numbers as qq_numbers, optional_id, optional_number,
    optional_text,
};
