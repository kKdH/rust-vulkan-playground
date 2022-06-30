use std::iter::{Copied, Enumerate};
use std::ops::RangeFrom;
use std::slice::Iter;
use nom::{AsBytes, Compare, CompareResult, InputIter, InputLength, InputTake, Needed, Slice};
use crate::blend::parse::Data;


#[derive(Debug, Copy, Clone)]
pub struct Input<'a> {
    pub data: &'a [u8],
    pub position: usize,
}

impl<'a> Input<'a> {

    pub fn new(data: &[u8]) -> Input {
        Input {
            data,
            position: 0
        }
    }

    pub fn new_with_position(data: &[u8], position: usize) -> Input {
        Input {
            data,
            position
        }
    }
}

impl <'a> AsBytes for Input<'a> {
    fn as_bytes(&self) -> &[u8] {
        self.data
    }
}

impl <'a> Compare<Data<'a>> for Input<'a> {

    fn compare(&self, tag: Data<'a>) -> CompareResult {

        if tag.len() > self.data.len() {
            return CompareResult::Incomplete
        }

        for (a, b) in tag.iter().zip(self.data) {
            if a != b {
                return CompareResult::Error
            }
        }

        CompareResult::Ok
    }

    fn compare_no_case(&self, _tag: Data<'a>) -> CompareResult {
        unimplemented!()
    }
}

impl <'a> InputLength for Input<'a> {
    fn input_len(&self) -> usize {
        self.data.len()
    }
}

impl <'a> InputTake for Input<'a> {

    fn take(&self, count: usize) -> Self {
        Input::new_with_position(&self.data[..count], self.position + count)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let head = &self.data[..count];
        let tail = &self.data[count..];
        (Input::new_with_position(tail, self.position + count), Input::new_with_position(head, self.position))
    }
}

impl<'a> InputIter for Input<'a> {
    type Item = Input<'a>;
    type Iter = Enumerate<Self::IterElem>;
    type IterElem = Copied<Iter<'a, Input<'a>>>;

    fn iter_indices(&self) -> Self::Iter {
        unimplemented!()
    }

    fn iter_elements(&self) -> Self::IterElem {
        unimplemented!()
    }

    fn position<P>(&self, _predicate: P) -> Option<usize> where P: Fn(Self::Item) -> bool {
        unimplemented!()
    }

    fn slice_index(&self, count: usize) -> ::std::result::Result<usize, Needed> {
        if self.data.len() >= count {
            Ok(count)
        } else {
            Err(Needed::new(count - self.data.len()))
        }
    }
}

impl<'a> Slice<RangeFrom<usize>> for Input<'a> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        Input::new_with_position(&self.data[range.start..], self.position + range.start)
    }
}
