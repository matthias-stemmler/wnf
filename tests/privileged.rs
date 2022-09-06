// TODO

testutils::system_tests![assert_is_privileged, failing_test];

fn assert_is_privileged() {
    assert!(wnf::can_create_permanent_shared_objects().unwrap());
}

fn failing_test() {
    assert_eq!(1, 2);
}
