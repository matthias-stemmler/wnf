use crate::WnfChangeStamp;

pub trait WnfCallback<T, Args, Return = ()> {
    fn call(&mut self, data: T, change_stamp: WnfChangeStamp) -> Return;
}

impl<F, T, Return> WnfCallback<T, (), Return> for F
where
    F: FnMut() -> Return,
{
    fn call(&mut self, _: T, _: WnfChangeStamp) -> Return {
        self()
    }
}

impl<F, T, Return> WnfCallback<T, (T,), Return> for F
where
    F: FnMut(T) -> Return,
{
    fn call(&mut self, data: T, _: WnfChangeStamp) -> Return {
        self(data)
    }
}

impl<F, T, Return> WnfCallback<T, (T, WnfChangeStamp), Return> for F
where
    F: FnMut(T, WnfChangeStamp) -> Return,
{
    fn call(&mut self, data: T, change_stamp: WnfChangeStamp) -> Return {
        self(data, change_stamp)
    }
}
