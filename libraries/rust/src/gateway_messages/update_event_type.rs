#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum UpdateEventType {
    ADD_ITEM = 0x0_u8, 
    REMOVE_ITEM = 0x1_u8, 
    CLEAR_CACHE = 0x2_u8, 
    DELETE_CACHE = 0x3_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for UpdateEventType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::ADD_ITEM, 
            0x1_u8 => Self::REMOVE_ITEM, 
            0x2_u8 => Self::CLEAR_CACHE, 
            0x3_u8 => Self::DELETE_CACHE, 
            _ => Self::NullVal,
        }
    }
}
