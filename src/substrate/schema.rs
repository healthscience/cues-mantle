#[derive(Clone, Copy)]
pub enum SubstrateSlot {
    ClearColorR = 0,
    ClearColorG = 1,
    ClearColorB = 2,
    ClearColorA = 3,
    LogoOriginX = 4,
    LogoOriginY = 5,
    LogoScaleW = 6,
    LogoScaleH = 7,
}

impl SubstrateSlot {
    pub fn index(&self) -> usize {
        *self as usize
    }
}
