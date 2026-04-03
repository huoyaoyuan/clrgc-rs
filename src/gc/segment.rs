pub struct Segment {
    pub size: usize,
    pub data: Box<[u8]>
}

impl Segment {
    pub fn new(size: usize) -> Self {
        Self { size, data: Box::from_iter(vec![0; size]) }
    }
}
