use std::cell::RefCell;
use std::marker::PhantomData;
use std::{fmt, mem};

use blend_inspect_rs::{BlendFile, BlendSource, Data, FileBlock, parse, Version};
use thiserror::Error;
use crate::blend::{GeneratedBlendStruct, PointerLike};

pub struct Reader<'a> {
    data: Data<'a>,
    blend: BlendFile,
}

impl <'a> Reader<'a> {

    pub fn structs<A>(&self) -> Result<StructIter<A>, ReadError>
    where A: 'a + GeneratedBlendStruct {
        self.assert_version(A::BLEND_VERSION, || {
            let views: Vec<FileBlockView<'a, A>> = self.blend.blocks.iter()
                .filter_map(|block| {
                    if block.sdna == A::STRUCT_INDEX {
                        Some(FileBlockView::new(self.data, &block))
                    } else {
                        None
                    }
                }).collect();
            StructIter::new(views)
        })
    }

    pub fn deref<A,  B>(&self, pointer: A) -> Result<StructIter<B>, ReadError>
    where A: PointerLike<B>,
          B: 'a + GeneratedBlendStruct {
        let lookup = pointer.address().map(|address| self.blend.look_up(address)).flatten();
        match lookup {
            None => Err(ReadError::InvalidPointerError),
            Some(block) => {
                self.assert_version(B::BLEND_VERSION, || {
                    StructIter::new(vec![FileBlockView::new(self.data, block)])
                })
            }
        }
    }

    fn assert_version<F, R>(&self, version: Version, function: F) -> Result<R, ReadError>
    where F: Fn() -> R {
        if self.blend.header.version == version {
            Ok(function())
        }
        else {
            Err(ReadError::VersionMismatchError {
                expected_version: version,
                actual_version: self.blend.header.version,
            })
        }
    }
}

pub struct StructIter<'a, A>
where A: GeneratedBlendStruct {
    views: Vec<FileBlockView<'a, A>>,
    view_index: RefCell<usize>,
    struct_index: RefCell<usize>,
    phantom: PhantomData<&'a A>,
}

impl <'a, A> StructIter<'a, A>
where A: GeneratedBlendStruct {

    fn new(views: Vec<FileBlockView<'a, A>>) -> StructIter<'a, A>{
        StructIter {
            views,
            view_index: RefCell::new(0),
            struct_index: RefCell::new(0),
            phantom: PhantomData::default(),
        }
    }

    pub fn find<P>(&self, predicate: P) -> Option<&'a A>
    where P: Fn(&'a A) -> bool {
        let mut view_index = 0;
        let mut struct_index = 0;
        while view_index < self.views.len() {
            if struct_index < self.views[view_index].count {
                let viewed_struct = self.views[view_index].view(struct_index);
                if predicate(viewed_struct) {
                    return Some(viewed_struct)
                }
                else {
                    struct_index += 1;
                }
            }
            else {
                struct_index = 0;
                view_index += 1;
            }
        }
        None
    }

    pub fn first(&self) -> Option<&'a A> {
        if self.views.len() > 0 && self.views[0].count > 0 {
            Some(self.views[0].view(0))
        }
        else {
            None
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

impl <'a, A> fmt::Debug for StructIter<'a, A>
where A: 'a + GeneratedBlendStruct {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("StructIter")
            .field("views", &self.views)
            .field("view_index", &self.view_index)
            .field("struct_index", &self.struct_index)
            .finish()
    }
}

struct FileBlockView<'a, A> {
    data: Data<'a>,
    count: usize,
    size: usize,
    phantom: PhantomData<&'a A>,
}

impl <'a, A> fmt::Debug for FileBlockView<'a, A>
    where A: 'a + GeneratedBlendStruct {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("FileBlockView")
            .field("data", &self.data)
            .field("count", &self.count)
            .field("size", &self.size)
            .finish()
    }
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

#[derive(Error, Debug)]
pub enum ReadError {

    #[error("Expected blend version {expected_version} but actual version is {actual_version}!")]
    VersionMismatchError {
        expected_version: Version,
        actual_version: Version
    },

    #[error("Invalid pointer address!")]
    InvalidPointerError,
}

pub fn read<'a, A>(source: A) -> Result<Reader<'a>, ReadError>
    where A: BlendSource<'a> {

    let data = source.data();
    let blend: BlendFile = parse(source.data()).unwrap();

    Ok(Reader { data, blend })
}

#[cfg(test)]
mod test {
    use hamcrest2::{assert_that, err, is, HamcrestMatcher};
    use crate::blend::{read, NameLike};
    use crate::blender3_0::{Object};

    #[test]
    fn test_that_structs_should_fail_on_version_mismatch() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        assert_that!(reader.structs::<crate::blender2_7::Object>(), is(err()))
    }

    #[test]
    fn test_that_deref_should_fail_on_version_mismatch() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let cube: &Object = reader.structs::<Object>().unwrap()
            .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
            .unwrap();

        assert_that!(reader.deref(&cube.data.cast_to::<crate::blender2_7::Mesh>()), is(err()))
    }

    #[test]
    fn test_that_find_returns_the_first_struct_matching_the_specified_predicate() {
        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let iter = reader.structs::<Object>().unwrap();
        let result = iter.find(|object| object.id.name.to_name_str_unchecked() == "Cube");

        assert_that!(result.is_some(), is(true))
    }

    #[test]
    fn test_that_find_returns_none_if_no_struct_matches_the_specified_predicate() {
        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let iter = reader.structs::<Object>().unwrap();
        let result = iter.find(|object| object.id.name.to_name_str_unchecked() == "Fubar");

        assert_that!(result.is_none(), is(true))
    }

    #[test]
    fn test_that_first_returns_the_first_struct() {
        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let result = reader.structs::<Object>().unwrap().first();

        assert_that!(result.is_some(), is(true))
    }
}