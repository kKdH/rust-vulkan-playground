use std::convert::TryInto;
use std::num::NonZeroUsize;
use nom::branch::alt;

use nom::bytes::complete::{tag, take};
use nom::character::complete::{alpha1, alphanumeric1, digit1};
use nom::combinator::{all_consuming, map, recognize, value};
use nom::{Finish, IResult};
use nom::multi::{many0, many0_count};
use nom::sequence::{delimited, pair, terminated, tuple};
use thiserror::Error;

use crate::blend::analyse::input::Input;
use crate::blend::Blend;
use crate::blend::parse::{Dna, DnaField, DnaStruct, DnaType, FileBlock, Identifier};

type Result<'a, A> = ::core::result::Result<A, AnalyseError>;

#[derive(Error, Debug, PartialEq)]
enum AnalyseError {

    #[error("Dna not found!")]
    DnaNotFound,

    #[error("A field with name '{name}' could not be found!")]
    FieldNotFound { name: String },

    #[error("The value '{value}' could not be parsed as field name!")]
    MalformedFieldName { value: String },

    #[error("A field name with '{index}' does not exist!")]
    InvalidFieldNameIndex { index: usize },

    #[error("A type with '{index}' does not exist!")]
    InvalidTypeIndex { index: usize },

    #[error("Unknown type '{value}'!")]
    UnknownType { value: String },
}

mod input {
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
}

#[derive(Debug, Clone, PartialEq)]
enum Type {
    Char,
    UChar,
    Short,
    UShort,
    Int,
    Int8,
    Int64,
    UInt64,
    Long,
    ULong,
    Float,
    Double,
    Void,
    Struct { name: String, size: usize },
    Pointer { base_type: Box<Type>, size: usize },
    Array { base_type: Box<Type>, length: usize }
}

impl Type {

    pub fn size(&self) -> usize {
        Type::compute_size(self)
    }

    fn compute_size(data_type: &Self) -> usize {
        match data_type {
            Type::Char => 1,
            Type::UChar => 1,
            Type::Short => 2,
            Type::UShort => 2,
            Type::Int => 4,
            Type::Int8 => 1,
            Type::Int64 => 8,
            Type::UInt64 => 8,
            Type::Long => 8,
            Type::ULong => 8,
            Type::Float => 4,
            Type::Double => 4,
            Type::Void => 0,
            Type::Struct { name, size } => *size,
            Type::Pointer { base_type, size } => *size,
            Type::Array { base_type, length } => length * Type::compute_size(base_type),
        }
    }

    pub fn from<'a>(dna_type: &'a DnaType, _: &'a Dna) -> Result<'a, Type> {
        let name = &dna_type.name;
        let parse_primitive_type = {
            alt((
                value(
                    Type::Char,
                    all_consuming(tag("char"))
                ),
                value(
                    Type::UChar,
                    all_consuming(tag("uchar"))
                ),
                value(
                    Type::Short,
                    all_consuming(tag("short"))
                ),
                value(
                    Type::UShort,
                    all_consuming(tag("ushort"))
                ),
                value(
                    Type::Int,
                    all_consuming(tag("int"))
                ),
                value(
                    Type::Int8,
                    all_consuming(tag("int8_t"))
                ),
                value(
                    Type::Int64,
                    all_consuming(tag("int64_t"))
                ),
                value(
                    Type::UInt64,
                    all_consuming(tag("uint64_t"))
                ),
                value(
                    Type::Long,
                    all_consuming(tag("long"))
                ),
                value(
                    Type::ULong,
                    all_consuming(tag("ulong"))
                ),
                value(
                    Type::Float,
                    all_consuming(tag("float"))
                ),
                value(
                    Type::Double,
                    all_consuming(tag("double"))
                ),
                value(
                    Type::Void,
                    all_consuming(tag("void"))
                )
            ))
        };
        let parse_struct_type = {
            map(
                all_consuming(
                    recognize(
                        pair(
                            alt((alpha1, tag("_"))),
                            many0(alt((alphanumeric1, tag("_"))))
                        )
                    ),
                ),
                | parsed_type: &str | {
                    Type::Struct {
                        name: String::from(parsed_type),
                        size: dna_type.size,
                    }
                })
        };

        let mut result: IResult<&str, Type> = alt((
            parse_primitive_type,
            parse_struct_type,
        ))(name);

        match result {
            Ok((_, parsed_type)) => Ok(parsed_type),
            Err(_) => Err(AnalyseError::UnknownType { value: String::from(name) })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Field {
    name: String,
    data_type: Type,
}

impl Field {

    pub fn from<'a>(dna_field: &DnaField, dna: &Dna) -> Result<'a, Field> {
        let field_name = dna.field_name_of(dna_field)
            .ok_or(AnalyseError::InvalidFieldNameIndex { index: dna_field.name_index })?;
        let base_type = dna.type_of(dna_field)
            .ok_or(AnalyseError::InvalidTypeIndex { index: dna_field.type_index })?;

        Type::from(&base_type, dna).and_then(|base_type| {
            Field::parse_field_name(field_name, &base_type)
                .map(|(parsed_name, parsed_type)| {
                    Field {
                        data_type: parsed_type,
                        name: String::from(field_name)
                    }
                })
        })
    }

    fn parse_field_name<'a>(input: &'a str, base_type: &'a Type) -> Result<'a, (String, Type)> {
        let result: IResult<&str, (String, Type)> = {
            all_consuming(
                map(
                    tuple((
                        many0_count(tag("*")),
                        recognize(
                            pair(
                                alt((alpha1, tag("_"))),
                                many0(alt((alphanumeric1, tag("_"))))
                            )
                        ),
                        many0(
                            delimited(
                                tag("["),
                                digit1,
                                tag("]")
                            )
                        )
                    )),
                    |(pointers, name, arrays): (usize, &str, Vec<&str>)| {
                        let result_type = Clone::clone(base_type);
                        let result_type = if pointers > 0 {
                            (0..pointers).fold(result_type, |result, _| {
                                Type::Pointer { base_type: Box::new(result), size: 4 } // FIXME: Determine the correct pointer size.
                            })
                        } else {
                            result_type
                        };
                        let result_type = if arrays.len() > 0 {
                            arrays.iter()
                                .rev()
                                .fold(result_type, |result, array_size| {
                                    Type::Array { base_type: Box::new(result), length: array_size.parse::<usize>().unwrap() }
                                })
                        } else {
                            result_type
                        };

                        (String::from(name), result_type)
                    }
                )
            )(input)
        };
        match result {
            Ok((_, parsed)) => Ok(parsed),
            Err(_) => Err(AnalyseError::MalformedFieldName { value: String::from(input) })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Struct {
    name: String,
    fields: Vec<Field>,
    size: usize,
}

impl Struct {

    pub fn from<'a>(dna_struct: &'a DnaStruct, dna: &'a Dna) -> Result<'a, Struct> {

        let (name, size) = dna.type_of(dna_struct)
            .map(|dna_type| (Clone::clone(&dna_type.name), dna_type.size))
            .ok_or(AnalyseError::InvalidTypeIndex { index: dna_struct.type_index })?;

        let fields = dna_struct.fields.iter().map(|dna_field| {
            Field::from(dna_field, dna)
        }).collect::<Result<Vec<Field>>>()?;

        Ok(Struct { name, fields, size })
    }
}


fn analyse(blend: Blend, input: Input) {

    let dna_block = blend.blocks_by_identifier(Identifier::DNA).unwrap()[0];
    let location = dna_block.data_location();
    let start: usize = location.into();
    let end: usize = start + dna_block.length;

    // let data = read(dna_block, input);
    //
    // println!("x: {:?}", data);
}

// fn read(input: Input, file_block: FileBlock) -> Result<Data> {
//     let start = file_block.data_location();
//     let end = start + file_block.length;
//     Ok(&input.data[start..end])
// }

#[cfg(test)]
mod test {
    use std::fmt::{Display, Formatter};
    use std::process::id;

    use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is, ok, type_of};
    use nom::{Err, IResult, Parser};
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::character::complete::{alpha1, alphanumeric1, digit1, satisfy};
    use nom::combinator::{all_consuming, map, recognize, value};
    use nom::error::{ErrorKind, ParseError};
    use nom::multi::{fold_many1, many0, many0_count};
    use nom::sequence::{delimited, pair, tuple};

    use crate::blend::analyse::{analyse, AnalyseError, Field, Input, Result, Struct};
    use crate::blend::Blend;
    use crate::blend::parse::{Dna, DnaField, DnaStruct, DnaType, FileBlock, Identifier, parse};

    #[test]
    fn test_analyse_dna() {
        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let blend = parse(blend_data.as_slice()).unwrap();

        // let input = Input::new(blend, blend_file.as_slice());
        // let dna = analyse_dna(input).ok().unwrap();
        // analyse(blend, blend_file.as_slice());

        let id_type = blend.dna().types.iter()
            .enumerate()
            .find(|(_, tpe)| String::from("ID") == tpe.name)
            .unwrap();

        let id_struct = blend.dna().structs.iter()
            .enumerate()
            .find(|(_, stc)| id_type.0 == stc.type_index)
            .unwrap();

        // let objects = blend.blocks_by_identifier(Identifier::OB).unwrap();
        // objects.iter().for_each(|object| {
        //     print_info(blend.dna(), object)
        // });

        // let meshes = blend.blocks_by_identifier(Identifier::ME).unwrap();
        // meshes.iter().for_each(|mesh| {
        //     print_info(blend.dna(), mesh)
        // });

        let scenes = blend.blocks_by_identifier(Identifier::SC).unwrap();
        scenes.iter().for_each(|scene| {
            let dna = blend.dna();
            let dna_struct = dna.struct_of(scene).unwrap();
            let dna_type = dna.type_of(dna_struct).unwrap();

            let s = Struct::from(dna_struct, dna).unwrap();
            println!("{:#?}", s)
        });
    }

    mod type_spec {
        use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

        use crate::blend::analyse::{AnalyseError, Type};
        use crate::blend::parse::{Dna, DnaType};

        #[test]
        fn test_parse_type() {
            let dna = Dna {
                field_names: vec![],
                types: vec![],
                structs: vec![]
            };
            let matrix: Vec<(&str, std::result::Result<Type, AnalyseError>)> = vec![
                ("char", Ok(Type::Char)),
                ("uchar", Ok(Type::UChar)),
                ("short", Ok(Type::Short)),
                ("ushort", Ok(Type::UShort)),
                ("int", Ok(Type::Int)),
                ("long", Ok(Type::Long)),
                ("ulong", Ok(Type::ULong)),
                ("float", Ok(Type::Float)),
                ("double", Ok(Type::Double)),
                ("int64_t", Ok(Type::Int64)),
                ("uint64_t", Ok(Type::UInt64)),
                ("void", Ok(Type::Void)),
                ("int8_t", Ok(Type::Int8)),
                ("Material", Ok(Type::Struct { name: String::from("Material"), size: 0 })),
                ("CustomData_MeshMasks", Ok(Type::Struct { name: String::from("CustomData_MeshMasks"), size: 0 })),
                ("", Err(AnalyseError::UnknownType { value: String::from("") })),
            ];

            matrix.iter().for_each(|(name, expected)| {
                let dna_type = DnaType {
                    name: String::from(*name),
                    size: 0
                };
                assert_that!(Type::from(&dna_type, &dna).as_ref(), is(equal_to(expected.as_ref())))
            });
        }
    }

    mod field_spec {
        use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

        use crate::blend::analyse::{Result, Type, Field};
        use crate::blend::parse::{Dna, DnaField, DnaType};

        #[test]
        fn test_parse_primitive_field() {
            setup(|fixture|{
                {
                    let field = DnaField {
                        name_index: 0,
                        type_index: 0
                    };
                    assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                        Field {
                            data_type: Type::Float,
                            name: String::from("x"),
                        }
                    ))));
                }
                {
                    let field = DnaField {
                        name_index: 4,
                        type_index: 0
                    };
                    assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                        Field {
                            data_type: Type::Float,
                            name: String::from("val2"),
                        }
                    ))));
                }
                {
                    let field = DnaField {
                        name_index: 5,
                        type_index: 1
                    };
                    assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                        Field {
                            data_type: Type::Void,
                            name: String::from("_pad"),
                        }
                    ))));
                }
            });
        }

        #[test]
        fn test_parse_struct_field() {
            setup(|fixture| {
                let field = DnaField {
                    name_index: 3,
                    type_index: 3
                };
                assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                    Field {
                        data_type: Type::Struct { name: String::from("ID"), size: 0 },
                        name: String::from("id"),
                    }
                ))));
            });
        }

        #[test]
        fn test_parse_pointer_field() {
            setup(|fixture| {
                {
                    let field = DnaField {
                        name_index: 1,
                        type_index: 1
                    };
                    assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                        Field {
                            data_type: Type::Pointer { base_type: Box::new(Type::Void), size: 4 },
                            name: String::from("*next"),
                        }
                    ))));
                }
                {
                    let field = DnaField {
                        name_index: 9,
                        type_index: 4
                    };
                    assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                        Field {
                            data_type: Type::Pointer { base_type: Box::new(Type::Char), size: 4 },
                            name: String::from("*ui_data"),
                        }
                    ))));
                }
            });
        }

        #[test]
        fn test_parse_pointer_of_pointer_of_primitive_field() {
            setup(|fixture| {
                let field = DnaField {
                    name_index: 2,
                    type_index: 1
                };
                assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                    Field {
                        data_type: Type::Pointer { base_type: Box::new(Type::Pointer { base_type: Box::new(Type::Void), size: 4 }), size: 4 },
                        name: String::from("**mat"),
                    }
                ))));
            })
        }

        #[test]
        fn test_parse_pointer_of_pointer_of_struct_field() {
            setup(|fixture| {
                let field = DnaField {
                    name_index: 2,
                    type_index: 2
                };
                assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                    Field {
                        data_type: Type::Pointer {
                            base_type: Box::new(
                                Type::Pointer {
                                    base_type: Box::new(
                                        Type::Struct {
                                            name: String::from("Material"),
                                            size: 0
                                        }
                                    ),
                                    size: 4
                                }
                            ),
                            size: 4
                        },
                        name: String::from("**mat"),
                    }
                ))));
            })
        }

        #[test]
        fn test_parse_array_of_primitives_field() {
            setup(|fixture| {
                let field = DnaField {
                    name_index: 7,
                    type_index: 4
                };
                assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                    Field {
                        data_type: Type::Array { base_type: Box::new(Type::Char), length: 1024 },
                        name: String::from("name[1024]"),
                    }
                ))));
            });
        }

        #[test]
        fn test_parse_array_of_pointers_of_primitives_field() {
            setup(|fixture| {
                let field = DnaField {
                    name_index: 10,
                    type_index: 4
                };
                assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                    Field {
                        data_type: Type::Array { base_type: Box::new(Type::Pointer { base_type: Box::new(Type::Char), size: 4 }), length: 4 },
                        name: String::from("*_pad_1[4]"),
                    }
                ))));
            });
        }

        #[test]
        fn test_parse_array_of_array_of_primitives_field() {
            setup(|fixture| {
                let field = DnaField {
                    name_index: 11,
                    type_index: 0
                };
                assert_that!(Field::from(&field, &fixture.dna), is(equal_to(Ok(
                    Field {
                        data_type: Type::Array { base_type: Box::new(Type::Array { base_type: Box::new(Type::Float), length: 3 }), length: 4 },
                        name: String::from("scale[4][3]"),
                    }
                ))));
            });
        }

        struct Fixture {
            dna: Dna
        }

        fn setup(test_code: fn(Fixture)) {
            let fixture = Fixture {
                dna: Dna {
                    field_names: vec![
                        "x",
                        "*next",
                        "**mat",
                        "id",
                        "val2",
                        "_pad",
                        "rna_prop_type",
                        "name[1024]",
                        "*rect[2]",
                        "*ui_data",
                        "*_pad_1[4]",
                        "scale[4][3]",
                    ].into_iter().map(|name| String::from(name)).collect(),
                    types: vec![
                        DnaType::new("float", 0),
                        DnaType::new("void", 0),
                        DnaType::new("Material", 0),
                        DnaType::new("ID", 0),
                        DnaType::new("char", 0),
                    ],
                    structs: vec![]
                }
            };

            test_code(fixture);
        }
    }

    mod struct_spec {
        use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

        use crate::blend::parse::{Dna, DnaField, DnaStruct, DnaType};
        use crate::blend::analyse::{Type, Field, Struct};

        #[test]
        fn test_parse_struct() {

            let dna = Dna {
                field_names: vec![
                    "*next",
                    "name[24]",
                    "id",
                    "scale[3]",
                ].into_iter().map(|name| String::from(name)).collect(),
                types: vec![
                    DnaType::new("ID", 42),
                    DnaType::new("char", 1),
                    DnaType::new("Mesh", 73),
                    DnaType::new("float", 4),
                ],
                structs: vec![
                    DnaStruct {
                        type_index: 3,
                        fields: vec![
                            DnaField { name_index: 0, type_index: 0 },
                            DnaField { name_index: 1, type_index: 1 },
                        ]
                    }
                ]
            };

            let dna_struct = DnaStruct {
                type_index: 2,
                fields: vec![
                    DnaField { name_index: 2, type_index: 0 },
                    DnaField { name_index: 3, type_index: 3 },
                ]
            };

            assert_that!(Struct::from(&dna_struct, &dna), is(equal_to(Ok(
                Struct {
                    name: String::from("Mesh"),
                    fields: vec![
                        Field { name: String::from("id"), data_type: Type::Struct { name: String::from("ID"), size: 42 } },
                        Field { name: String::from("scale[3]"), data_type: Type::Array { base_type: Box::new(Type::Float), length: 3 } }
                    ],
                    size: 73
                }
            ))));
        }
    }

    #[test]
    fn test_read_block() {
        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let blend = parse(blend_data.as_slice()).unwrap();

        let input = Input::new(&blend, &blend_data);

        let file_block = blend.blocks_by_identifier(Identifier::ME).unwrap()[0];

        let file_block_struct = blend.dna().struct_of(&file_block).unwrap();
        let id_dna_type = blend.dna().type_of(&file_block_struct.fields[0]).unwrap();

        let id_dna_struct = blend.dna().structs.iter().find(| strct| strct.type_index == 34 ).unwrap();

        // blend.dna().types.iter().for_each(|t| {
        //     println!("  {:?}", t.name);
        // });

        blend.dna().field_names.iter().for_each(|name| {
            println!("  {:?}", name)
        });

        println!("struct: {:?}", file_block_struct);
        println!("id_dna_type: {:?}", id_dna_type);
        println!("id_dna_struct: {:?}", id_dna_struct);


        // let (acc, data) = read_field("id")(input, file_block).unwrap();


    }

    fn read(data: Vec<u8>, blend: &Blend, block: &FileBlock, ) {

        let dna = blend.dna();
        // let input = Input::new(blend, &data);

        // let field = read_field("id")(input).unwrap();
    }

    fn read_field(name: &'static str) -> impl Fn(Input, FileBlock) -> Result<Field> {
        move |input: Input, block: FileBlock| {



            Err(AnalyseError::FieldNotFound { name: String::from(name) })
        }
    }
}
