use super::sent;

#[test]
fn platform_seconds_are_written_the_way_the_core_reads_them() {
    assert_eq!(
        sent(1_760_934_600).as_deref(),
        Some("2025-10-20T04:30:00.000Z")
    );
    assert_eq!(sent(0), None);
    assert_eq!(sent(-5), None);
}
