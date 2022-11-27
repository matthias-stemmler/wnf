use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;

use wnf::{ChangeStamp, DataAccessor, OwnedState, SeenChangeStamp};

#[test]
fn data_type_can_be_inferred_from_apply_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.apply(|x: ()| x).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_apply_boxed_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.apply_boxed(|x: Box<()>| x).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_try_apply_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.try_apply(Ok::<(), TestError>).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_try_apply_boxed_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.try_apply_boxed(Ok::<Box<()>, TestError>).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_get_call() {
    let state = OwnedState::create_temporary().unwrap();
    let _: () = state.get().unwrap();
}

#[test]
fn data_type_can_be_inferred_from_get_boxed_call() {
    let state = OwnedState::create_temporary().unwrap();
    let _: Box<()> = state.get_boxed().unwrap();
}

#[test]
fn data_type_can_be_inferred_from_query_call() {
    let state = OwnedState::create_temporary().unwrap();
    let _: () = state.query().unwrap().into_data();
}

#[test]
fn data_type_can_be_inferred_from_query_boxed_call() {
    let state = OwnedState::create_temporary().unwrap();
    let _: Box<()> = state.query_boxed().unwrap().into_data();
}

#[test]
fn data_type_can_be_inferred_from_replace_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.replace(&()).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_replace_boxed_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.replace_boxed(&()).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_subscribe_call() {
    let state = OwnedState::create_temporary().unwrap();
    let _ = state
        .subscribe(|_: DataAccessor<()>| {}, SeenChangeStamp::None)
        .unwrap();
}

#[test]
fn data_type_can_be_inferred_from_set_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.set(&()).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_update_call() {
    let state = OwnedState::create_temporary().unwrap();
    state.update(&(), ChangeStamp::initial()).unwrap();
}

#[tokio::test]
async fn data_type_can_be_inferred_from_wait_until_async() {
    let state = OwnedState::create_temporary().unwrap();
    let _: () = state.wait_until_async(|_| true).await.unwrap();
}

#[tokio::test]
async fn data_type_can_be_inferred_from_wait_until_boxed_async() {
    let state = OwnedState::create_temporary().unwrap();
    let _: Box<()> = state.wait_until_boxed_async(|_| true).await.unwrap();
}

#[test]
fn data_type_can_be_inferred_from_wait_until_blocking() {
    let state = OwnedState::create_temporary().unwrap();
    let _: () = state.wait_until_blocking(|_| true, Duration::ZERO).unwrap();
}

#[test]
fn data_type_can_be_inferred_from_wait_until_boxed_blocking() {
    let state = OwnedState::create_temporary().unwrap();
    let _: Box<()> = state.wait_until_boxed_blocking(|_| true, Duration::ZERO).unwrap();
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for TestError {}
