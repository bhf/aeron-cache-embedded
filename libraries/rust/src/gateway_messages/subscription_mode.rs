#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SubscriptionMode {
    FULL = 0x0_u8, 
    PATCH = 0x1_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for SubscriptionMode {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::FULL, 
            0x1_u8 => Self::PATCH, 
            _ => Self::NullVal,
        }
    }
}
