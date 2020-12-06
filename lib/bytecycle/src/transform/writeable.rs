use crate::transform::Offset;

pub trait Writeable {
    fn write(&mut self, offset: Offset, amount: Offset, buffer: &[u8]);
}

impl Writeable for Vec<u8> {
    fn write(&mut self, offset: usize, amount: usize, buffer: &[u8]) {
        for index in 0..amount {
            self.push(buffer[index]);
        }
    }
}
