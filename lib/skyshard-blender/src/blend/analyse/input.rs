use crate::blend::Blend;

pub type Data<'a> = &'a [u8];

pub struct Input<'a> {
    pub blend: &'a Blend,
    pub data: Data<'a>
}

impl <'a> Input<'a> {
    pub fn new(blend: &'a Blend, data: Data<'a>) -> Self {
        Self {
            blend,
            data
        }
    }
}
