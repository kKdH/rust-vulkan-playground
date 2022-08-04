use std::cell::RefCell;
use std::marker::PhantomData;
use std::mem;

use blend_inspect_rs::{BlendFile, BlendSource, Data, FileBlock, parse};
use thiserror::Error;
use crate::blend::{GeneratedBlendStruct, PointerLike};

pub struct Reader<'a> {
    data: Data<'a>,
    blend: BlendFile,
}

impl <'a> Reader<'a> {

    pub fn structs<A>(&self) -> StructIter<A>
        where A: 'a + GeneratedBlendStruct {
        let views: Vec<FileBlockView<'a, A>> = self.blend.blocks.iter()
            .filter_map(|block| {
                if block.sdna == A::STRUCT_INDEX {
                    Some(FileBlockView::new(self.data, &block))
                }
                else {
                    None
                }
            }).collect();
        StructIter::new(views)
    }

    pub fn deref<A,  B>(&self, pointer: A) -> StructIter<B>
    where A: PointerLike<B>,
          B: 'a + GeneratedBlendStruct {
        let views = if let Some(address) = pointer.address(){
            self.blend.look_up(address)
                .iter()
                .map(|block| FileBlockView::new(self.data, block))
                .collect()
        } else {
            Vec::new()
        };

        StructIter::new(views)
    }
}

struct FileBlockView<'a, A> {
    data: Data<'a>,
    count: usize,
    size: usize,
    phantom: PhantomData<&'a A>,
}

impl <'a, A> FileBlockView<'a, A>
where A: 'a + GeneratedBlendStruct {

    fn new(data: Data<'a>, block: &FileBlock) -> FileBlockView<'a, A> {
        let start_offset = block.data_location();
        let end_offset = block.data_location() + (mem::size_of::<A>() * block.count);
        Self {
            data: &data[start_offset..end_offset],
            count: block.count,
            size: mem::size_of::<A>(),
            phantom: PhantomData::default(),
        }
    }

    fn view(&self, index: usize) -> &'a A {
        let offset = self.size * index;
        let (before, body, after) = unsafe {
            self.data[offset..(offset + self.size)].align_to::<A>()
        };
        if before.len() == 0 && body.len() == 1 && after.len() == 0 {
            &body[0]
        }
        else {
            panic!("Failed to align struct '{}' (prefix={}, suffix={}, alignments={})! ", A::STRUCT_NAME, before.len(), after.len(), body.len())
        }
    }
}

pub struct StructIter<'a, A> {
    views: Vec<FileBlockView<'a, A>>,
    view_index: RefCell<usize>,
    struct_index: RefCell<usize>,
    phantom: PhantomData<&'a A>,
}

impl <'a, A> StructIter<'a, A> {
    fn new(views: Vec<FileBlockView<'a, A>>) -> StructIter<'a, A>{
        StructIter {
            views,
            view_index: RefCell::new(0),
            struct_index: RefCell::new(0),
            phantom: PhantomData::default(),
        }
    }
}

impl <'a, A> Iterator for &StructIter<'a, A>
    where A: 'a + GeneratedBlendStruct {

    type Item = &'a A;

    fn next(&mut self) -> Option<Self::Item> {
        let mut view_index = self.view_index.borrow_mut();
        let mut struct_index = self.struct_index.borrow_mut();
        if *view_index < self.views.len() {
            if *struct_index < self.views[*view_index].count {
                let result = self.views[*view_index].view(*struct_index);
                *struct_index += 1;
                Some(result)
            }
            else {
                *view_index += 1;
                *struct_index = 0;

                if *view_index < self.views.len() && self.views[*view_index].count > 0 {
                    let result = self.views[*view_index].view(*struct_index);
                    *struct_index += 1;
                    Some(result)
                }
                else {
                    None
                }
            }
        }
        else {
            None
        }
    }
}

#[derive(Error, Debug)]
pub enum ReadError {

}

pub fn read<'a, A>(source: A) -> Result<Reader<'a>, ReadError>
    where A: BlendSource<'a> {

    let data = source.data();
    let blend: BlendFile = parse(source.data()).unwrap();

    Ok(Reader { data, blend })
}
