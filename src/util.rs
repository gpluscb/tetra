use std::fmt::{Debug, Formatter};

pub struct OmitDebug;

impl Debug for OmitDebug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[omitted]")
    }
}
