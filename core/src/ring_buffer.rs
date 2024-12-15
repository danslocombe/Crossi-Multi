use std::mem::MaybeUninit;

pub struct RingBuffer<T> {
    //buffer: Box<[Option<T>]>,
    buffer: Box<[MaybeUninit<T>]>,
    current_pos: usize,
    push_count: usize,
}

impl<T> Default for RingBuffer<T> {
    fn default() -> Self {
        Self::new(0)
    }
}

impl<T: Clone> RingBuffer<T> {
    pub fn new_with_value(size: usize, value: T) -> Self {
        let mut buffer = Vec::new();
        let mut push_count = 0;
        for _i in 0..size {
            buffer.push(MaybeUninit::new(value.clone()));
            push_count += 1;
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            current_pos: 0,
            push_count,
        }
    }
}

impl<T> RingBuffer<T> {
    pub fn new(size: usize) -> Self {
        let mut buffer = Vec::with_capacity(size);
        for _i in 0..size {
            buffer.push(MaybeUninit::uninit());
            //buffer.push(None);
        }

        Self {
            buffer: buffer.into_boxed_slice(),
            current_pos: 0,
            push_count: 0,
        }
    }

    pub fn push(&mut self, x: T) {
        self.incr_current_pos();
        //self.buffer[self.current_pos] = Some(x);
        let mut maybe_x = MaybeUninit::new(x);
        std::mem::swap(&mut maybe_x, &mut self.buffer[self.current_pos]);
        self.push_count += 1;
        if (self.push_count > self.buffer.len()) {
            // We have wrapped around at least once.
            // There is an object at the slot we are writing to.
            // We should cleanup this old object
            unsafe { maybe_x.assume_init_drop() };
        }
    }

    pub fn incr_current_pos(&mut self) {
        self.current_pos += 1;
        if (self.current_pos == self.buffer.len()) {
            self.current_pos = 0;
        }
    }

    pub fn get(&self, offset: i32) -> &T {
        let pos = self.pos_wrapping(offset);
        unsafe { self.buffer[pos].assume_init_ref() }
        //self.buffer[pos].as_ref().unwrap()
    }

    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    pub fn pos_wrapping(&self, offset: i32) -> usize {
        // Assume: offset < buffersize
        let size = self.buffer.len() as i32;

        let mut pos = self.current_pos as i32 + offset;
        if (pos < 0) {
            pos += size;
        } else if (pos >= size) {
            pos -= size;
        }

        pos as usize
    }
}