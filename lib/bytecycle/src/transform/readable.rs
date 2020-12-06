use crate::transform::Offset;

pub trait Readable {
    fn read(&self, offset: Offset, amount: Offset, buffer: &mut [u8]);
}

impl Readable for Vec<u8> {
    fn read(&self, offset: usize, amount: usize, buffer: &mut [u8]) {
        for index in 0..amount {
            buffer[index] = self[offset + index]
        }
    }
}
