#[test]
fn can_create_permanent_shared_objects_succeeds() {
    // We cannot assert on the actual boolean return value as it depends on the privileges with which the test is run
    assert!(wnf::can_create_permanent_shared_objects().is_ok());
}
