#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BulkOperationType {
    NONE = 0x0_u8, 
    CREATE_CACHE = 0x1_u8, 
    ADD_ITEM = 0x2_u8, 
    REMOVE_ITEM = 0x3_u8, 
    CLEAR_CACHE = 0x4_u8, 
    GET_ITEM = 0x5_u8, 
    DELETE_CACHE = 0x6_u8, 
    PATCH_ITEM = 0x7_u8, 
    CREATE_COUNTER_CACHE = 0x8_u8, 
    ADD_COUNTER = 0x9_u8, 
    REMOVE_COUNTER = 0xa_u8, 
    CLEAR_COUNTER_CACHE = 0xb_u8, 
    GET_COUNTER = 0xc_u8, 
    DELETE_COUNTER_CACHE = 0xd_u8, 
    INCREMENT_COUNTER = 0xe_u8, 
    DECREMENT_COUNTER = 0xf_u8, 
    SET_COUNTER = 0x10_u8, 
    CANCEL_ITEM = 0x11_u8, 
    CANCEL_COUNTER = 0x12_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for BulkOperationType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::NONE, 
            0x1_u8 => Self::CREATE_CACHE, 
            0x2_u8 => Self::ADD_ITEM, 
            0x3_u8 => Self::REMOVE_ITEM, 
            0x4_u8 => Self::CLEAR_CACHE, 
            0x5_u8 => Self::GET_ITEM, 
            0x6_u8 => Self::DELETE_CACHE, 
            0x7_u8 => Self::PATCH_ITEM, 
            0x8_u8 => Self::CREATE_COUNTER_CACHE, 
            0x9_u8 => Self::ADD_COUNTER, 
            0xa_u8 => Self::REMOVE_COUNTER, 
            0xb_u8 => Self::CLEAR_COUNTER_CACHE, 
            0xc_u8 => Self::GET_COUNTER, 
            0xd_u8 => Self::DELETE_COUNTER_CACHE, 
            0xe_u8 => Self::INCREMENT_COUNTER, 
            0xf_u8 => Self::DECREMENT_COUNTER, 
            0x10_u8 => Self::SET_COUNTER, 
            0x11_u8 => Self::CANCEL_ITEM, 
            0x12_u8 => Self::CANCEL_COUNTER, 
            _ => Self::NullVal,
        }
    }
}
