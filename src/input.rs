use serde::{Deserialize, Serialize};

// Button bitfield constants
pub const BTN_LEFT: u8 = 1 << 0;
pub const BTN_RIGHT: u8 = 1 << 1;
pub const BTN_JUMP: u8 = 1 << 2;
pub const BTN_CROUCH: u8 = 1 << 3;
pub const BTN_LIGHT: u8 = 1 << 4;
pub const BTN_HEAVY: u8 = 1 << 5;
pub const BTN_SPECIAL: u8 = 1 << 6;
pub const BTN_BLOCK: u8 = 1 << 7;

/// 8 bytes: frame number + button bitfield + padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub struct InputFrame {
    pub frame_num: u32,
    pub buttons: u8,
    pub _pad: [u8; 3],
}

impl InputFrame {
    pub const fn new(frame_num: u32, buttons: u8) -> Self {
        Self {
            frame_num,
            buttons,
            _pad: [0; 3],
        }
    }

    #[inline]
    pub const fn pressed(&self, btn: u8) -> bool {
        self.buttons & btn != 0
    }
}

/// Fixed-size ring buffer for input history (120 deep).
#[derive(Clone, Copy)]
pub struct InputHistory {
    pub buf: [InputFrame; 120],
    pub head: usize,
    pub count: usize,
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHistory {
    pub const fn new() -> Self {
        Self {
            buf: [InputFrame::new(0, 0); 120],
            head: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, input: InputFrame) {
        self.buf[self.head] = input;
        self.head = (self.head + 1) % 120;
        if self.count < 120 {
            self.count += 1;
        }
    }

    /// Get the most recent input (0 = newest).
    pub fn get_recent(&self, age: usize) -> Option<InputFrame> {
        if age >= self.count {
            return None;
        }
        let idx = (self.head + 120 - 1 - age) % 120;
        Some(self.buf[idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_frame_size() {
        assert_eq!(core::mem::size_of::<InputFrame>(), 8);
    }

    #[test]
    fn input_history_push_get() {
        let mut hist = InputHistory::new();
        for i in 0..5 {
            hist.push(InputFrame::new(i, i as u8));
        }
        assert_eq!(hist.get_recent(0).unwrap().frame_num, 4);
        assert_eq!(hist.get_recent(4).unwrap().frame_num, 0);
        assert!(hist.get_recent(5).is_none());
    }

    #[test]
    fn input_history_wraps() {
        let mut hist = InputHistory::new();
        for i in 0..130u32 {
            hist.push(InputFrame::new(i, 0));
        }
        assert_eq!(hist.count, 120);
        assert_eq!(hist.get_recent(0).unwrap().frame_num, 129);
        assert_eq!(hist.get_recent(119).unwrap().frame_num, 10);
    }

    #[test]
    fn button_pressed() {
        let f = InputFrame::new(0, BTN_JUMP | BTN_LIGHT);
        assert!(f.pressed(BTN_JUMP));
        assert!(f.pressed(BTN_LIGHT));
        assert!(!f.pressed(BTN_HEAVY));
    }
}
