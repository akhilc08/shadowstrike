/// Fixed-size ring buffer with no heap allocation.
#[derive(Debug, Clone, Copy)]
pub struct RingBuffer<T: Copy, const N: usize> {
    data: [T; N],
    frames: [u64; N],
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    pub fn new(default: T) -> Self {
        RingBuffer {
            data: [default; N],
            frames: [u64::MAX; N],
        }
    }

    pub fn write(&mut self, frame: u64, item: T) {
        let idx = (frame as usize) % N;
        self.data[idx] = item;
        self.frames[idx] = frame;
    }

    pub fn read(&self, frame: u64) -> Option<&T> {
        let idx = (frame as usize) % N;
        if self.frames[idx] == frame {
            Some(&self.data[idx])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut buf: RingBuffer<i32, 4> = RingBuffer::new(0);

        buf.write(0, 10);
        buf.write(1, 20);
        buf.write(2, 30);
        buf.write(3, 40);

        assert_eq!(buf.read(0), Some(&10));
        assert_eq!(buf.read(1), Some(&20));
        assert_eq!(buf.read(2), Some(&30));
        assert_eq!(buf.read(3), Some(&40));

        // Overwrite oldest
        buf.write(4, 50);
        assert_eq!(buf.read(4), Some(&50));
        assert_eq!(buf.read(0), None); // overwritten

        // Still valid
        assert_eq!(buf.read(1), Some(&20));
        assert_eq!(buf.read(2), Some(&30));
        assert_eq!(buf.read(3), Some(&40));

        // Non-existent frame
        assert_eq!(buf.read(100), None);
    }
}
