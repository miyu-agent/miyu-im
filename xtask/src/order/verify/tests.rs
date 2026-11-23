//! 核对排上了（施工 LW-IM-8）：排在开头的过、散着的报错、空清单和找不到的报错、同名的局部函数一起算。

use super::check;

/// 清单的三个函数排在代码段开头（0x1000 起），后面是没执行过的。
const ORDERED: &str = "\
0000000000001000 0000000000000100 T hot_a
0000000000001100 0000000000000100 t hot_b
0000000000001200 0000000000000100 t hot_c
0000000000001300 0000000000010000 T cold_a
0000000000011300 0000000000010000 T cold_b
";

/// 同样几个函数散在代码段里。
const SCATTERED: &str = "\
0000000000001000 0000000000000100 T hot_a
0000000000001100 0000000000010000 T cold_a
0000000000011100 0000000000000100 t hot_b
0000000000011200 0000000000010000 T cold_b
0000000000021200 0000000000000100 t hot_c
";

const LIST: &str = "hot_a\nhot_b\n\nhot_c\nhot_a\n";

#[test]
fn functions_at_the_start_pass() {
    let placed = check(ORDERED, LIST).expect("排上了");
    assert_eq!((placed.listed, placed.found), (3, 3));
    assert_eq!((placed.bytes, placed.near), (0x300, 0x300));
}

#[test]
fn scattered_functions_fail() {
    let error = check(SCATTERED, LIST).expect_err("散着");
    assert!(error.contains("九成"), "{error}");
}

#[test]
fn empty_and_foreign_lists_fail() {
    assert!(check(ORDERED, "\n \n").expect_err("空").contains("空"));
    assert!(
        check(ORDERED, "other\n")
            .expect_err("找不到")
            .contains("找不到")
    );
}

#[test]
fn local_functions_sharing_a_name_count_together() {
    let nm = "\
0000000000001000 0000000000000100 t dup
0000000000001100 0000000000000100 t dup
0000000000001200 0000000000010000 T cold
";
    let placed = check(nm, "dup\n").expect("排上了");
    assert_eq!(placed.bytes, 0x200);
}

#[test]
fn a_list_recorded_from_another_build_fails() {
    // 一半的名字这一份程序里没有（照别的编法编的程序录的）：排上的只是零头，不算。
    let error = check(ORDERED, "hot_a\ngone_1\ngone_2\n").expect_err("对不上");
    assert!(error.contains("找到"), "{error}");
}
