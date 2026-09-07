#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BooleanType {
    F = 0x0_u8, 
    T = 0x1_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for BooleanType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::F, 
            0x1_u8 => Self::T, 
            _ => Self::NullVal,
        }
    }
}
