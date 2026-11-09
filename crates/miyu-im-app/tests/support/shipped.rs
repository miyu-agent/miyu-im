//! 出厂的几样（施工 I-3 补从 `mod.rs` 挪来：那个文件到了行数上限）：`bridge.json`、清单里的默认值、出厂的场所规则。

use miyu_im::rules::Factory;
use miyu_im::settings::Defaults;
use miyu_im::tuning::Tuning;

use super::{package, package_in, resources};

/// 出厂的 `bridge.json`。
pub fn tuning() -> Tuning {
    Tuning::load(&package().data()).expect("出厂的 bridge.json 读得出来")
}

/// 出厂的清单里 NapCat 端口的默认值（施工 O-20）。
pub fn defaults() -> Defaults {
    let driver = (
        miyu_im_onebot::onebot::PLATFORM.name(),
        miyu_im_onebot::settings::NAMES,
    );
    Defaults::load(&package_in(resources()), &[driver]).expect("出厂的清单读得出来")
}

/// 出厂的场所规则、出厂参数、违规词表（施工 O-21）。
pub fn factory() -> Factory {
    factory_at(&package_in(resources())).expect("出厂的读得出来")
}

/// 照包文件夹 `package` 读出厂的（同桥起来时）：驱动交的工具（施工 T-1）一起读。
pub fn factory_at(package: &miyu_im::package::Package) -> Result<Factory, Vec<miyu_chat::Problem>> {
    use miyu_im::driver::Driver;
    let onebot = miyu_im_onebot::OneBot;
    let toolsets = onebot
        .toolset(&tuning())
        .map(|set| (onebot.platform(), set));
    Factory::load(package, toolsets.into_iter().collect())
}
