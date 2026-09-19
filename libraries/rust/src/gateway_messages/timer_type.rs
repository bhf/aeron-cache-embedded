#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TimerType {
    CACHE = 0x0_u8, 
    COUNTER = 0x1_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for TimerType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::CACHE, 
            0x1_u8 => Self::COUNTER, 
            _ => Self::NullVal,
        }
    }
}
