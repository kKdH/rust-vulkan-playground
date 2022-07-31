use std::marker::PhantomData;

pub struct Void;

pub struct Pointer<T, const SIZE: usize> {
    pub value: [u8; SIZE],
    phantom: PhantomData<T>
}

impl <T, const SIZE: usize> Pointer<T, SIZE> {
    pub fn new(value: [u8; SIZE]) -> Self {
        Pointer {
            value,
            phantom: Default::default()
        }
    }
}

pub struct Function<const SIZE: usize> {
    pub value: [u8; SIZE]
}

#[cfg(feature = "blender2_7")]
pub mod blender2_7;

#[cfg(feature = "blender2_9")]
pub mod blender2_9;

#[cfg(feature = "blender3_0")]
pub mod blender3_0;

#[cfg(test)]
mod test {
    use std::mem;

    use hamcrest2::{assert_that, equal_to, is};
    use hamcrest2::HamcrestMatcher;

    use blender_inspect::{analyse, Mode, parse};

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let (file_header, file_blocks, dna) = parse(blend_data.as_slice()).unwrap();
        let structure = analyse(&file_header, &file_blocks, &dna, Mode::All).unwrap();

        let mesh_struct_size = mem::size_of::<crate::blender::blender3_0::Mesh>();
        let mesh_structure = structure.find_struct_by_name("Mesh").unwrap();
        let mesh_dna_struct = dna.find_struct_by_name("Mesh").unwrap();
        let mesh_dna_type = dna.find_type_of(mesh_dna_struct).unwrap();

        // mesh_dna_struct.fields.iter()
        //     .map(|dna_field| {
        //         (dna.find_field_name_of(dna_field).unwrap(),
        //         dna.find_type_of(dna_field).unwrap())
        //     })
        //     .for_each(|(name, ty)| {
        //         println!("{}: {} ({})", name, ty.name, ty.size);
        //     });
        // dna.types.iter().for_each(|dna_type| {
        //     println!("dna_type: {}, {}", dna_type.name, dna_type.size)
        // });


        assert_that!(mesh_struct_size, is(equal_to(mesh_structure.size())));
        assert_that!(mesh_struct_size, is(equal_to(mesh_dna_type.size)));
    }
}
