/// Input bitmask:
/// bit 0=left, 1=right, 2=up(jump), 3=down(crouch),
/// 4=light_attack, 5=heavy_attack, 6=special, 7=block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Input(pub u8);

pub const NEUTRAL: Input = Input(0);

impl Input {
    pub fn is_left(self) -> bool {
        self.0 & (1 << 0) != 0
    }
    pub fn is_right(self) -> bool {
        self.0 & (1 << 1) != 0
    }
    pub fn is_up(self) -> bool {
        self.0 & (1 << 2) != 0
    }
    pub fn is_down(self) -> bool {
        self.0 & (1 << 3) != 0
    }
    pub fn is_light(self) -> bool {
        self.0 & (1 << 4) != 0
    }
    pub fn is_heavy(self) -> bool {
        self.0 & (1 << 5) != 0
    }
    pub fn is_special(self) -> bool {
        self.0 & (1 << 6) != 0
    }
    pub fn is_block(self) -> bool {
        self.0 & (1 << 7) != 0
    }
}
