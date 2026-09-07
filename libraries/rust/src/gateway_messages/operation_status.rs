#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OperationStatus {
    NONE = 0x0_u8, 
    SUCCESS = 0x1_u8, 
    ERROR = 0x2_u8, 
    UNKNOWN_CACHE = 0x3_u8, 
    UNKNOWN_KEY = 0x4_u8, 
    CACHE_EXISTS = 0x5_u8, 
    DUPLICATE_SUBSCRIPTION = 0x6_u8, 
    UNKNOWN_SUBSCRIPTION = 0x7_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for OperationStatus {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::NONE, 
            0x1_u8 => Self::SUCCESS, 
            0x2_u8 => Self::ERROR, 
            0x3_u8 => Self::UNKNOWN_CACHE, 
            0x4_u8 => Self::UNKNOWN_KEY, 
            0x5_u8 => Self::CACHE_EXISTS, 
            0x6_u8 => Self::DUPLICATE_SUBSCRIPTION, 
            0x7_u8 => Self::UNKNOWN_SUBSCRIPTION, 
            _ => Self::NullVal,
        }
    }
}
