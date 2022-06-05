use crate::WnfChangeStamp;

pub trait WnfCallback<T, ArgsValid, ArgsInvalid, Return = ()> {
    fn call_valid(&mut self, data: T, change_stamp: WnfChangeStamp) -> Return;

    fn call_invalid(&mut self, change_stamp: WnfChangeStamp) -> Option<Return>;
}

impl<F, T, Return> WnfCallback<T, (), (), Return> for F
where
    F: FnMut() -> Return,
{
    fn call_valid(&mut self, _: T, _: WnfChangeStamp) -> Return {
        self()
    }

    fn call_invalid(&mut self, _: WnfChangeStamp) -> Option<Return> {
        None
    }
}

impl<F, T, Return> WnfCallback<T, (T,), (), Return> for F
where
    F: FnMut(T) -> Return,
{
    fn call_valid(&mut self, data: T, _: WnfChangeStamp) -> Return {
        self(data)
    }

    fn call_invalid(&mut self, _: WnfChangeStamp) -> Option<Return> {
        None
    }
}

impl<F, T, Return> WnfCallback<T, (T, WnfChangeStamp), (), Return> for F
where
    F: FnMut(T, WnfChangeStamp) -> Return,
{
    fn call_valid(&mut self, data: T, change_stamp: WnfChangeStamp) -> Return {
        self(data, change_stamp)
    }

    fn call_invalid(&mut self, _: WnfChangeStamp) -> Option<Return> {
        None
    }
}

impl<F, G, T, Return> WnfCallback<T, (), (), Return> for CatchInvalid<F, G>
where
    F: FnMut() -> Return,
    G: FnMut() -> Return,
{
    fn call_valid(&mut self, _: T, _: WnfChangeStamp) -> Return {
        (self.valid_handler)()
    }

    fn call_invalid(&mut self, _: WnfChangeStamp) -> Option<Return> {
        Some((self.invalid_handler)())
    }
}

impl<F, G, T, Return> WnfCallback<T, (), (WnfChangeStamp,), Return> for CatchInvalid<F, G>
where
    F: FnMut() -> Return,
    G: FnMut(WnfChangeStamp) -> Return,
{
    fn call_valid(&mut self, _: T, _: WnfChangeStamp) -> Return {
        (self.valid_handler)()
    }

    fn call_invalid(&mut self, change_stamp: WnfChangeStamp) -> Option<Return> {
        Some((self.invalid_handler)(change_stamp))
    }
}

impl<F, G, T, Return> WnfCallback<T, (T,), (), Return> for CatchInvalid<F, G>
where
    F: FnMut(T) -> Return,
    G: FnMut() -> Return,
{
    fn call_valid(&mut self, data: T, _: WnfChangeStamp) -> Return {
        (self.valid_handler)(data)
    }

    fn call_invalid(&mut self, _: WnfChangeStamp) -> Option<Return> {
        Some((self.invalid_handler)())
    }
}

impl<F, G, T, Return> WnfCallback<T, (T,), (WnfChangeStamp,), Return> for CatchInvalid<F, G>
where
    F: FnMut(T) -> Return,
    G: FnMut(WnfChangeStamp) -> Return,
{
    fn call_valid(&mut self, data: T, _: WnfChangeStamp) -> Return {
        (self.valid_handler)(data)
    }

    fn call_invalid(&mut self, change_stamp: WnfChangeStamp) -> Option<Return> {
        Some((self.invalid_handler)(change_stamp))
    }
}

impl<F, G, T, Return> WnfCallback<T, (T, WnfChangeStamp), (), Return> for CatchInvalid<F, G>
where
    F: FnMut(T, WnfChangeStamp) -> Return,
    G: FnMut() -> Return,
{
    fn call_valid(&mut self, data: T, change_stamp: WnfChangeStamp) -> Return {
        (self.valid_handler)(data, change_stamp)
    }

    fn call_invalid(&mut self, _: WnfChangeStamp) -> Option<Return> {
        Some((self.invalid_handler)())
    }
}

impl<F, G, T, Return> WnfCallback<T, (T, WnfChangeStamp), (WnfChangeStamp,), Return> for CatchInvalid<F, G>
where
    F: FnMut(T, WnfChangeStamp) -> Return,
    G: FnMut(WnfChangeStamp) -> Return,
{
    fn call_valid(&mut self, data: T, change_stamp: WnfChangeStamp) -> Return {
        (self.valid_handler)(data, change_stamp)
    }

    fn call_invalid(&mut self, change_stamp: WnfChangeStamp) -> Option<Return> {
        Some((self.invalid_handler)(change_stamp))
    }
}

#[derive(Debug)]
pub struct CatchInvalid<F, G> {
    valid_handler: F,
    invalid_handler: G,
}

pub trait CatchInvalidExt<ArgsInvalid, T>: Sized {
    fn catch_invalid<G>(self, invalid_handler: G) -> CatchInvalid<Self, G> {
        CatchInvalid {
            valid_handler: self,
            invalid_handler,
        }
    }
}

impl<F, T, Return> CatchInvalidExt<(), T> for F where F: FnMut() -> Return {}
impl<F, T, Return> CatchInvalidExt<(T,), T> for F where F: FnMut(T) -> Return {}
impl<F, T, Return> CatchInvalidExt<(T, WnfChangeStamp), T> for F where F: FnMut(T, WnfChangeStamp) -> Return {}
