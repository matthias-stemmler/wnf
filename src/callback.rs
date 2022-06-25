use crate::read::WnfReadError;
use crate::WnfChangeStamp;

pub trait WnfCallback<T, Args, Return = ()> {
    fn call(&mut self, result: Result<T, WnfReadError>, change_stamp: WnfChangeStamp) -> Option<Return>;
}

impl<F, T, Return> WnfCallback<T, (), Return> for F
where
    F: FnMut() -> Return,
{
    fn call(&mut self, result: Result<T, WnfReadError>, _: WnfChangeStamp) -> Option<Return> {
        match result {
            Ok(..) => Some(self()),
            Err(..) => None,
        }
    }
}

impl<F, T, Return> WnfCallback<T, (T,), Return> for F
where
    F: FnMut(T) -> Return,
{
    fn call(&mut self, result: Result<T, WnfReadError>, _: WnfChangeStamp) -> Option<Return> {
        match result {
            Ok(data) => Some(self(data)),
            Err(..) => None,
        }
    }
}

impl<F, T, Return> WnfCallback<T, (T, WnfChangeStamp), Return> for F
where
    F: FnMut(T, WnfChangeStamp) -> Return,
{
    fn call(&mut self, result: Result<T, WnfReadError>, change_stamp: WnfChangeStamp) -> Option<Return> {
        match result {
            Ok(data) => Some(self(data, change_stamp)),
            Err(..) => None,
        }
    }
}

#[derive(Debug)]
pub struct WnfCallbackMaybeInvalid<F>(F);

impl<F> From<F> for WnfCallbackMaybeInvalid<F> {
    fn from(f: F) -> Self {
        Self(f)
    }
}

impl<F, T, Return> WnfCallback<T, (), Return> for WnfCallbackMaybeInvalid<F>
where
    F: FnMut() -> Return,
{
    fn call(&mut self, _: Result<T, WnfReadError>, _: WnfChangeStamp) -> Option<Return> {
        Some((self.0)())
    }
}

impl<F, T, Return> WnfCallback<T, (T,), Return> for WnfCallbackMaybeInvalid<F>
where
    F: FnMut(Result<T, WnfReadError>) -> Return,
{
    fn call(&mut self, result: Result<T, WnfReadError>, _: WnfChangeStamp) -> Option<Return> {
        Some((self.0)(result))
    }
}

impl<F, T, Return> WnfCallback<T, (T, WnfChangeStamp), Return> for WnfCallbackMaybeInvalid<F>
where
    F: FnMut(Result<T, WnfReadError>, WnfChangeStamp) -> Return,
{
    fn call(&mut self, result: Result<T, WnfReadError>, change_stamp: WnfChangeStamp) -> Option<Return> {
        Some((self.0)(result, change_stamp))
    }
}
