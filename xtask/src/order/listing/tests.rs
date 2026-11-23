//! 日志换函数清单（施工 LW-IM-8）：符号表、装载起点、指令行、照先后去重、代码段以外的不算。

use super::{Listing, address, base, functions};

const NM: &str = "\
0000000000001000 0000000000000010 T _start
0000000000001010 0000000000000020 t _ZN4main4mainE
0000000000001030 0000000000000010 W _ZN4core3fmt5writeE
0000000000001040 0000000000000000 t no_size
0000000000002000 0000000000000008 R some_data
0000000000001050 0000000000000030 T _ZN4miyu5serveE
";

const LOG: &str = "\
host mmap_min_addr=0x10000
Locating guest address space @ 0x0
page layout changed following mmap
start            end              size             prot
0000555555556000-0000555556506000 0000000000fb0000 ---
ffffffffff600000-ffffffffff601000 0000000000001000 --x
----------------
IN:
0x00007fe96fc4d7e0:  48 89 e7                 movq     %rsp, %rdi
0x0000555555557050:  f3 0f 1e fa              endbr64
0x0000555555557011:  55                       pushq    %rbp
0x0000555555557055:  48 89 e5                 movq     %rsp, %rbp
0x0000555555557000:  f3 0f 1e fa              endbr64
";

#[test]
fn only_sized_code_symbols_are_functions_sorted_by_address() {
    let found = functions(NM);
    let names: Vec<&str> = found
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "_start",
            "_ZN4main4mainE",
            "_ZN4core3fmt5writeE",
            "_ZN4miyu5serveE"
        ]
    );
}

#[test]
fn the_base_is_the_first_mapping_under_the_layout_header() {
    assert_eq!(base(LOG), Some(0x5555_5555_6000));
    assert_eq!(base("IN:\n0x1:  90 nop\n"), None);
}

#[test]
fn instruction_lines_give_their_address() {
    assert_eq!(
        address("0x0000555555557050:  f3 0f 1e fa  endbr64"),
        Some(0x5555_5555_7050)
    );
    assert_eq!(address("IN:"), None);
    assert_eq!(address("0000555555556000-0000555556506000 ---"), None);
}

#[test]
fn functions_are_listed_once_in_first_execution_order_outside_code_ignored() {
    let found = functions(NM);
    let mut listing = Listing::new(&found);
    let base = base(LOG).expect("有起点");
    LOG.lines().for_each(|line| listing.line(base, line));
    let names: Vec<&str> = listing.names().collect();
    assert_eq!(names, ["_ZN4miyu5serveE", "_ZN4main4mainE", "_start"]);
    assert_eq!(listing.count(), 3);
    assert_eq!(listing.bytes(), 0x30 + 0x20 + 0x10);
    // 第二份日志接着收：收过的不再记，新的接在后面。
    listing.line(base, "0x0000555555557030:  90 nop");
    listing.line(base, "0x0000555555557050:  90 nop");
    let names: Vec<&str> = listing.names().collect();
    assert_eq!(names.last(), Some(&"_ZN4core3fmt5writeE"));
    assert_eq!(listing.count(), 4);
}
