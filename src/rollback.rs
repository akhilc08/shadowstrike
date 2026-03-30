use crate::game_state::GameState;
use crate::input::InputFrame;

/// Generic fixed-size ring buffer. All const-generic, no heap.
#[derive(Clone, Copy)]
pub struct RingBuffer<T: Copy + Default, const N: usize> {
    pub buf: [T; N],
    pub head: usize,
    pub count: usize,
}

impl<T: Copy + Default, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            buf: [T::default(); N],
            head: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.buf[self.head] = item;
        self.head = (self.head + 1) % N;
        if self.count < N {
            self.count += 1;
        }
    }

    /// Get item by age (0 = most recent).
    pub fn get_recent(&self, age: usize) -> Option<&T> {
        if age >= self.count {
            return None;
        }
        let idx = (self.head + N - 1 - age) % N;
        Some(&self.buf[idx])
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// 8-deep snapshot buffer for rollback.
pub type SnapshotBuffer = RingBuffer<GameState, 8>;

/// 120-deep input buffer per player.
pub type InputBuffer = RingBuffer<InputFrame, 120>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_basic() {
        let mut rb = RingBuffer::<u32, 4>::new();
        rb.push(10);
        rb.push(20);
        rb.push(30);
        assert_eq!(*rb.get_recent(0).unwrap(), 30);
        assert_eq!(*rb.get_recent(2).unwrap(), 10);
        assert!(rb.get_recent(3).is_none());
    }

    #[test]
    fn ring_buffer_wrap() {
        let mut rb = RingBuffer::<u32, 3>::new();
        for i in 0..10u32 {
            rb.push(i);
        }
        assert_eq!(rb.len(), 3);
        assert_eq!(*rb.get_recent(0).unwrap(), 9);
        assert_eq!(*rb.get_recent(1).unwrap(), 8);
        assert_eq!(*rb.get_recent(2).unwrap(), 7);
    }

    #[test]
    fn snapshot_buffer_stores_gamestate() {
        let mut sb = SnapshotBuffer::new();
        let gs = GameState::initial();
        sb.push(gs);
        assert_eq!(sb.get_recent(0).unwrap().p1.health, 1000);
    }
}
