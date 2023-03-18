use serde::Serialize;

pub struct RealtimeGraph
{
    data : Ringbuffer,
    min : f32,
    max : f32,
    count : usize,
}

#[derive(Debug, Serialize)]
pub struct RealtimeGraphSnapshot
{
    data : Vec<f32>,
    x_min : i32,
    y_min : f32,
    y_max : f32,
}

impl RealtimeGraph
{
    pub fn new(lookback_in_ticks : usize) -> Self {
        Self {
            data : Ringbuffer::new(lookback_in_ticks),
            min: 0.0,
            max: 0.0,
            count : 0,
        }
    }

    pub fn push(&mut self, value : f32) {
        self.data.push(value);
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.count += 1;
    }

    pub fn repeat(&mut self) {
        let top = self.data.get(0);
        self.push(top);
    }

    pub fn get_all(&self) -> Vec<f32> {
        let mut all = Vec::with_capacity(self.data.size());

        let to_index = self.count.min(self.data.size());
        for i in 0..to_index {
            let offset = to_index - i - 1;
            let sample = self.data.get(-(offset as i32));
            all.push(self.normalize_sample(sample));
        }

        all
    }

    fn normalize_sample(&self, sample : f32) -> f32 {
        if (self.max == self.min)
        {
            sample - self.min
        }
        else
        {
            (sample - self.min) / (self.max - self.min)
        }
    }

    pub fn snapshot(&self) -> RealtimeGraphSnapshot
    {
        let data = self.get_all();
        RealtimeGraphSnapshot
        {
            x_min : self.count as i32 - data.len() as i32,
            y_min : self.min,
            y_max : self.max,
            data,
        }
    }
}

pub struct Ringbuffer
{
    buffer : Vec<f32>,
    pos : usize,
}

impl Ringbuffer
{
    pub fn new(size : usize) -> Self {
        assert!(size > 0);

        let mut buffer = Vec::with_capacity(size);
        for _ in 0..size {
            buffer.push(0.0);
        }

        Self {
            pos : buffer.len() - 1,
            buffer,
        }
    }

    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    pub fn push(&mut self, value : f32)
    {
        self.pos = (self.pos + 1) % self.buffer.len();
        self.buffer[self.pos] = value;
    }

    pub fn get(&self, offset : i32) -> f32 {
        self.buffer[self.index_from_offset(offset)]
    }

    fn index_from_offset(&self, offset : i32) -> usize {
        assert!(offset.abs() < self.buffer.len() as i32);

        let size = self.buffer.len() as i32;

        let mut pos_with_offset = self.pos as i32 + offset;

        if (pos_with_offset < 0)
        {
            pos_with_offset += size;
        }
        else if (pos_with_offset > size)
        {
            pos_with_offset -= size;
        }

        pos_with_offset as usize
    }
}