use crate::bytes::NoUninit;

pub trait WnfWrite {}

impl<T> WnfWrite for T where T: NoUninit + ?Sized {}
