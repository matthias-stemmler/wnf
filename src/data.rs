use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WnfChangeStamp(u32);

impl WnfChangeStamp {
    pub fn initial() -> Self {
        Self::default()
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut u32 {
        &mut self.0
    }
}

impl From<u32> for WnfChangeStamp {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<WnfChangeStamp> for u32 {
    fn from(WnfChangeStamp(value): WnfChangeStamp) -> Self {
        value
    }
}

impl Display for WnfChangeStamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WnfStampedData<T> {
    data: T,
    change_stamp: WnfChangeStamp,
}

impl<T> WnfStampedData<T> {
    pub fn from_data_change_stamp(data: T, change_stamp: impl Into<WnfChangeStamp>) -> Self {
        Self {
            data,
            change_stamp: change_stamp.into(),
        }
    }

    pub fn into_data_change_stamp(self) -> (T, WnfChangeStamp) {
        (self.data, self.change_stamp)
    }

    pub fn into_data(self) -> T {
        self.data
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn change_stamp(&self) -> WnfChangeStamp {
        self.change_stamp
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub(crate) enum WnfNameInfoClass {
    StateNameExist = 0,
    SubscribersPresent = 1,
    IsQuiescent = 2,
}
