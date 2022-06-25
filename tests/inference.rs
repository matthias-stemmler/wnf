use wnf::{OwnedWnfState, WnfChangeStamp, WnfDataAccessor};

#[test]
fn data_type_can_be_inferred_from_set_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    state.set(42u32).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_update_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    state.update(42u32, WnfChangeStamp::initial()).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_get_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    let _: () = state.get().unwrap();
}

#[test]
fn data_type_can_be_inferred_from_get_boxed_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    let _: Box<()> = state.get_boxed().unwrap();
}

#[test]
fn data_type_can_be_inferred_from_query_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    let _: () = state.query().unwrap().into_data();
}

#[test]
fn data_type_can_be_inferred_from_query_boxed_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    let _: Box<()> = state.query_boxed().unwrap().into_data();
}

#[test]
fn data_type_can_be_inferred_from_apply_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    state.apply(|x: (), _| x).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_apply_boxed_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    state.apply_boxed(|x: Box<()>, _| x).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_try_apply_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    state.try_apply(|x: (), _| Ok::<_, TestError>(x)).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_try_apply_boxed_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    state.try_apply_boxed(|x: Box<()>, _| Ok::<_, TestError>(x)).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_subscribe_call() {
    let state = OwnedWnfState::create_temporary().unwrap();
    state
        .subscribe(WnfChangeStamp::initial(), Box::new(|_: WnfDataAccessor<()>, _| {}))
        .unwrap();
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;
