mod check;

use std::{fmt, mem};
use std::error::Error;
use std::marker::PhantomData;
use std::ops::Range;

use thiserror::Error;

use blend_inspect_rs::{BlendFile, BlendSource, Data, Endianness, FileBlock, parse, Version};

use crate::blend::{GeneratedBlendStruct, PointerLike};
use crate::blend::reader::check::{check_blend, check_same_type};
use crate::blend::traverse::{DoubleLinked, DoubleLinkedIter};

pub struct Reader<'a> {
    data: Data<'a>,
    blend: BlendFile,
}

impl <'a> Reader<'a> {

    /// Returns an iterator over all structs of the specified type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blend_rs::blend::{read, StructIter};
    /// use blend_rs::blender3_2::Mesh;
    ///
    /// let blend_data = std::fs::read("examples/example-3.2.blend")
    ///     .expect("Failed to open blend file!");
    ///
    /// let reader = read(&blend_data)
    ///     .expect("Failed to read blend file!");
    ///
    /// let meshes: StructIter<Mesh> = reader.iter::<Mesh>().expect("Failed to create an iterator!");
    /// ```
    pub fn iter<S>(&self) -> Result<StructIter<S>, ReadError>
    where S: 'a + GeneratedBlendStruct {
        check_blend::<S>(&self.blend)?;
        let views: Vec<FileBlockView<'a, S>> = self.blend.blocks.iter()
            .filter_map(|block| {
                if block.sdna == S::STRUCT_INDEX {
                    Some(FileBlockView::new(self.data, &block))
                } else {
                    None
                }
            }).collect();
        Ok(StructIter::new(views))
    }

    /// Dereferences the specified [PointerLike] and returns an iterator over the structs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blend_rs::blend::{read, PointerLike, NameLike, StructIter};
    /// use blend_rs::blender3_2::{Mesh, MLoop, MPoly};
    ///
    /// let blend_data = std::fs::read("examples/example-3.2.blend")
    ///     .expect("Failed to open blend file!");
    ///
    /// let reader = read(&blend_data)
    ///     .expect("Failed to read blend file!");
    ///
    /// let mesh: &Mesh = reader.iter::<Mesh>()
    ///    .expect("Failed to create an iterator!")
    ///    .find(|mesh| mesh.id.name.to_name_str_unchecked() == "Cube")
    ///    .expect("Failed to find 'Cube'!");
    ///
    /// let polygons: StructIter<MPoly> = reader.deref(&mesh.mpoly)
    ///     .expect("Failed to deref the mesh's polygons!");
    /// ```
    pub fn deref<P, S, const SIZE: usize>(&self, pointer: &P) -> Result<StructIter<S>, ReadError>
    where P: PointerLike<S, SIZE>,
          S: 'a + GeneratedBlendStruct {
        let block = self.look_up(pointer)?;
        check_blend::<S>(&self.blend)?;
        check_same_type(&self.blend, S::STRUCT_TYPE_INDEX, block)?;
        Ok(StructIter::new(vec![FileBlockView::new(self.data, block)]))
    }

    /// Dereferences the specified [PointerLike] and returns the struct.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blend_rs::blend::{read, PointerLike, NameLike};
    /// use blend_rs::blender3_2::{Object, Mesh};
    ///
    /// let blend_data = std::fs::read("examples/example-3.2.blend")
    ///     .expect("Failed to open blend file!");
    ///
    /// let reader = read(&blend_data)
    ///     .expect("Failed to read blend file!");
    ///
    /// let cube: &Object = reader.iter::<Object>()
    ///    .expect("Failed to create an iterator!")
    ///    .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
    ///    .expect("Failed to find 'Cube'!");
    ///
    /// let mesh: &Mesh = reader.deref_single(&cube.data.cast_to::<Mesh>())
    ///     .expect("Failed to deref the object's mesh!");
    /// ```
    pub fn deref_single<P, S, const SIZE: usize>(&self, pointer: &P) -> Result<&'a S, ReadError>
    where P: PointerLike<S, SIZE>,
          S: 'a + GeneratedBlendStruct {
        let block = self.look_up(pointer)?;
        check_blend::<S>(&self.blend)?;
        check_same_type(&self.blend, S::STRUCT_TYPE_INDEX, block)?;
        let file_block_view: FileBlockView<S> = FileBlockView::new(self.data, block);
        match file_block_view.len() {
            1 => Ok(file_block_view.view(0)),
            0 => Err(ReadError::NoSuchElementError),
            _ => Err(ReadError::MoreThanOneElementError)
        }
    }

    /// Dereferences the specified [PointerLike] and returns a slice of the raw data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blend_rs::blend::{read, PointerLike, NameLike};
    /// use blend_rs::blender3_2::{PackedFile};
    ///
    /// let blend_data = std::fs::read("examples/example-3.2.blend")
    ///     .expect("Failed to open blend file!");
    /// let reader = read(&blend_data)
    ///     .expect("Failed to read blend file!");
    ///
    /// let packed_file: &PackedFile = reader.iter::<PackedFile>()
    ///    .expect("Failed to create an iterator!")
    ///    .first()
    ///    .expect("No packed file!");
    ///
    /// let data = reader.deref_raw(&packed_file.data)
    ///     .expect("Failed to deref raw data!");
    /// ```
    pub fn deref_raw<P, T, const SIZE: usize>(&self, pointer: &P) -> Result<Data<'a>, ReadError>
    where P: PointerLike<T, SIZE> {
        let block = self.look_up(pointer)?;
        Ok(&self.data[block.data_location()..block.data_location() + block.length])
    }

    /// Dereferences the specified [PointerLike] and returns a sub-slice of the raw data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blend_rs::blend::{read, PointerLike, NameLike};
    /// use blend_rs::blender3_2::{PackedFile};
    ///
    /// let blend_data = std::fs::read("examples/example-3.2.blend")
    ///     .expect("Failed to open blend file!");
    /// let reader = read(&blend_data)
    ///     .expect("Failed to read blend file!");
    ///
    /// let packed_file: &PackedFile = reader.iter::<PackedFile>()
    ///    .expect("Failed to create an iterator!")
    ///    .first()
    ///    .expect("No packed file!");
    ///
    /// let magic_number = reader.deref_raw_range(&packed_file.data, 0..4 as usize)
    ///     .expect("Failed to deref raw data!");
    /// ```
    pub fn deref_raw_range<P, T, const SIZE: usize>(&self, pointer: &P, range: Range<usize>) -> Result<Data<'a>, ReadError>
    where P: PointerLike<T, SIZE> {
        self.deref_raw(pointer).map(|data| {
            &data[range.start..range.end]
        })
    }

    pub fn traverse<PD, D, PT, const SIZE: usize>(&self, pointer: &PD) -> Result<DoubleLinkedIter<D, PT, SIZE>, ReadError>
    where PD: PointerLike<D, SIZE>,
          D: 'a + DoubleLinked<PT, SIZE> + GeneratedBlendStruct,
          PT: PointerLike<D, SIZE> {

        self.deref_single(pointer).map(|first| {
            DoubleLinkedIter::new(self, first)
        })
    }

    fn look_up<A, B, const SIZE: usize>(&self, pointer: &A) -> Result<&FileBlock, ReadError>
    where A: PointerLike<B, SIZE> {
        let address = pointer.address();
        let lookup = address
            .map(|address| self.blend.look_up(address))
            .flatten();
        match lookup {
            None => {
                let address = address.map(|value| value.get()).unwrap_or(0usize);
                if address != 0 {
                    Err(ReadError::InvalidPointerAddressError { address })
                }
                else {
                    Err(ReadError::NullPointerError)
                }
            },
            Some(block) => {
                Ok(block)
            }
        }
    }
}

pub struct StructIter<'a, A>
where A: GeneratedBlendStruct {
    views: Vec<FileBlockView<'a, A>>,
    view_index: usize,
    struct_index: usize,
    length: usize,
    phantom: PhantomData<&'a A>,
}

impl <'a, A> StructIter<'a, A>
where A: GeneratedBlendStruct {

    fn new(views: Vec<FileBlockView<'a, A>>) -> StructIter<'a, A>{
        let length = views.iter().map(|view| view.len()).sum();
        StructIter {
            views,
            view_index: 0,
            struct_index: 0,
            length,
            phantom: PhantomData::default(),
        }
    }

    pub fn find<P>(&self, predicate: P) -> Option<&'a A>
    where P: Fn(&'a A) -> bool {
        let mut view_index = 0;
        let mut struct_index = 0;
        while view_index < self.views.len() {
            if struct_index < self.views[view_index].len() {
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
        if self.views.len() > 0 && self.views[0].len() > 0 {
            Some(self.views[0].view(0))
        }
        else {
            None
        }
    }
}

impl <'a, A> Iterator for StructIter<'a, A>
where A: 'a + GeneratedBlendStruct {

    type Item = &'a A;

    fn next(&mut self) -> Option<Self::Item> {
        if self.view_index < self.views.len() {
            if self.struct_index < self.views[self.view_index].len() {
                let result = self.views[self.view_index].view(self.struct_index);
                self.struct_index += 1;
                Some(result)
            }
            else {
                self.view_index += 1;
                self.struct_index = 0;

                if self.view_index < self.views.len() && self.views[self.view_index].len() > 0 {
                    let result = self.views[self.view_index].view(self.struct_index);
                    self.struct_index += 1;
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }
}

impl <'a, A> ExactSizeIterator for StructIter<'a, A>
where A: 'a + GeneratedBlendStruct {}

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

    fn len(&self) -> usize {
        self.count
    }
}

#[derive(Error, Debug)]
pub enum ReadError {

    #[error("Failed to parse blender data.")]
    ParseError {
        #[source]
        cause: Box<dyn Error>
    },

    #[error("Expected blend version {expected} but actual version is {actual}!")]
    VersionMismatchError {
        expected: Version,
        actual: Version
    },

    #[error("Expected pointer size {expected} but actual pointer size is {actual}!")]
    PointerSizeMismatchError {
        expected: usize,
        actual: usize
    },

    #[error("Expected endianness {expected} but actual endianness is {actual}!")]
    EndiannessMismatchError {
        expected: Endianness,
        actual: Endianness
    },

    #[error("Pointer address is null!")]
    NullPointerError,

    #[error("Invalid pointer address '{address}'!")]
    InvalidPointerAddressError { address: usize },

    #[error("Invalid pointer type!")]
    InvalidPointerTypeError { expected: String, actual: String },

    #[error("Contains no elements!")]
    NoSuchElementError,

    #[error("Contains more than one matching element!")]
    MoreThanOneElementError,
}

pub fn read<'a, A>(source: A) -> Result<Reader<'a>, ReadError>
    where A: BlendSource<'a> {

    let data = source.data();
    parse(source.data())
        .map(|blend| Reader { data, blend } )
        .map_err(|cause| { ReadError::ParseError { cause: Box::new(cause) } })
}

#[cfg(test)]
mod test {
    use hamcrest2::{assert_that, equal_to, err, HamcrestMatcher, is};

    use crate::blend::{NameLike, PointerLike, read};
    use crate::blender3_2::Object;

    #[test]
    fn test_that_iter_should_fail_on_version_mismatch() {

        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        assert_that!(reader.iter::<crate::blender2_79::Object>(), is(err()))
    }

    #[test]
    fn test_that_iter_should_fail_on_pointer_size_mismatch() {

        let blend_data = std::fs::read("gen/blender2_80.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        assert_that!(reader.iter::<crate::blender2_80x86::Object>(), is(err()))
    }

    #[test]
    fn test_that_deref_should_fail_on_version_mismatch() {

        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let cube: &Object = reader.iter::<Object>().unwrap()
            .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
            .unwrap();

        assert_that!(reader.deref(&cube.data.cast_to::<crate::blender2_79::Mesh>()), is(err()))
    }

    #[test]
    fn test_that_find_returns_the_first_struct_matching_the_specified_predicate() {
        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let iter = reader.iter::<Object>().unwrap();
        let result = iter.find(|object| object.id.name.to_name_str_unchecked() == "Cube");

        assert_that!(result.is_some(), is(true))
    }

    #[test]
    fn test_that_find_returns_none_if_no_struct_matches_the_specified_predicate() {
        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let iter = reader.iter::<Object>().unwrap();
        let result = iter.find(|object| object.id.name.to_name_str_unchecked() == "Fubar");

        assert_that!(result.is_none(), is(true))
    }

    #[test]
    fn test_that_first_returns_the_first_struct() {
        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let result = reader.iter::<Object>().unwrap().first();

        assert_that!(result.is_some(), is(true))
    }

    #[test]
    fn test_that_len_returns_the_number_of_struct() {
        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let result = reader.iter::<Object>().unwrap();

        assert_that!(result.len(), is(equal_to(3)))
    }
}
