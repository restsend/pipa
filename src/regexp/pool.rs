use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug)]
pub struct BacktrackState {
    pub pc: u32,

    pub pos: u32,

    pub counter: u32,

    pub capture_start: u32,
    pub capture_end: u32,
}

impl Default for BacktrackState {
    #[inline(always)]
    fn default() -> Self {
        Self {
            pc: 0,
            pos: 0,
            counter: 0,
            capture_start: u32::MAX,
            capture_end: u32::MAX,
        }
    }
}

pub struct BacktrackPool {
    inline: [MaybeUninit<BacktrackState>; 64],

    heap: Vec<BacktrackState>,

    used: usize,
}

impl BacktrackPool {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            inline: unsafe { MaybeUninit::uninit().assume_init() },
            heap: Vec::new(),
            used: 0,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.used = 0;
        self.heap.clear();
    }

    #[inline(always)]
    pub fn push(&mut self, state: BacktrackState) {
        if self.used < 64 {
            self.inline[self.used].write(state);
        } else {
            let heap_idx = self.used - 64;
            if heap_idx >= self.heap.len() {
                self.heap.push(state);
            } else {
                self.heap[heap_idx] = state;
            }
        }
        self.used += 1;
    }

    #[cfg(test)]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.used
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<BacktrackState> {
        if self.used == 0 {
            return None;
        }
        self.used -= 1;
        if self.used < 64 {
            Some(unsafe { self.inline[self.used].assume_init() })
        } else {
            Some(self.heap[self.used - 64])
        }
    }
}

impl Default for BacktrackPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backtrack_pool_inline() {
        let mut pool = BacktrackPool::new();

        for i in 0..64 {
            pool.push(BacktrackState {
                pc: i as u32,
                pos: i as u32,
                ..Default::default()
            });
        }

        for i in (0..64).rev() {
            let state = pool.pop().unwrap();
            assert_eq!(state.pc, i as u32);
        }

        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_backtrack_pool_heap() {
        let mut pool = BacktrackPool::new();

        for i in 0..100 {
            pool.push(BacktrackState {
                pc: i as u32,
                pos: i as u32,
                ..Default::default()
            });
        }

        for i in (0..100).rev() {
            let state = pool.pop().unwrap();
            assert_eq!(state.pc, i as u32);
        }

        assert_eq!(pool.len(), 0);
    }
}
