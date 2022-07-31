use std::collections::HashMap;
use std::num::NonZeroUsize;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, alphanumeric1, digit1};
use nom::combinator::{all_consuming, map, opt, recognize, value};
use nom::IResult;
use nom::multi::{many0, many0_count};
use nom::sequence::{delimited, pair, tuple};

use crate::analyse::{AnalyseError, Field, Mode, Result, Struct, Structure, Type};
use crate::parse::{Dna, DnaField, DnaStruct, DnaType, FileBlock, FileHeader, AddressTable};


pub fn analyse(file_header: &FileHeader, file_blocks: &Vec<FileBlock>, dna: &Dna, mode: Mode) -> Result<Structure> {

    let mut remaining: Vec<String> = {
        let result: Result<Vec<String>> = match mode {
            Mode::All => {
                dna.structs
                    .iter()
                    .map(|dna_struct| {
                        dna.find_type_of(dna_struct)
                            .map(|ty| Clone::clone(&ty.name))
                            .ok_or(AnalyseError::InvalidTypeIndex { index: dna_struct.type_index })
                    })
                    .collect()
            }
            Mode::RequiredOnly => {
                file_blocks.iter()
                    .map(|file_block| {
                        dna.find_struct_of(file_block)
                            .map(|dna_struct| {
                                dna.find_type_of(dna_struct)
                                    .map(|ty| Clone::clone(&ty.name))
                                    .ok_or(AnalyseError::InvalidTypeIndex { index: dna_struct.type_index })
                            })
                            .ok_or(AnalyseError::InvalidStructIndex { index: file_block.sdna })
                    })
                    .flatten()
                    .collect()
            }
        };

        result?
    };

    let mut structs: HashMap<String, Struct> = HashMap::new();

    while let Some(struct_name) = remaining.pop() {
        let dna_struct = dna.find_struct_by_name(&struct_name)
            .ok_or(AnalyseError::UnknownStruct { value: struct_name })?;
        let analysed_struct = analyse_struct(&dna_struct, &dna)?;

        for field in &analysed_struct.fields {
            match &field.data_type.base_type() {
                Type::Struct { name, size: _size } => {
                    if !structs.contains_key(name.as_str()) {
                        if !remaining.contains(&name) {
                            remaining.push(Clone::clone(name))
                        }
                    }
                },
                _ => {}
            }
        }

        structs.insert(Clone::clone(&analysed_struct.name), analysed_struct);
    }

    Ok(Structure::new(
        file_header.endianness,
        structs.into_iter().map(|(_, strct)| strct).collect(),
    ))
}

pub fn analyse_type(dna_type: &DnaType, _: &Dna) -> Result<Type> {
    let name = &dna_type.name;
    let parse_primitive_type = {
        alt((
            value(
                Type::Char,
                all_consuming(tag(Type::TYPE_NAME_CHAR))
            ),
            value(
                Type::UChar,
                all_consuming(tag(Type::TYPE_NAME_UCHAR))
            ),
            value(
                Type::Short,
                all_consuming(tag(Type::TYPE_NAME_SHORT))
            ),
            value(
                Type::UShort,
                all_consuming(tag(Type::TYPE_NAME_USHORT))
            ),
            value(
                Type::Int,
                all_consuming(tag(Type::TYPE_NAME_INT))
            ),
            value(
                Type::Int8,
                all_consuming(tag(Type::TYPE_NAME_INT8))
            ),
            value(
                Type::Int64,
                all_consuming(tag(Type::TYPE_NAME_INT64))
            ),
            value(
                Type::UInt64,
                all_consuming(tag(Type::TYPE_NAME_UINT64))
            ),
            value(
                Type::Long,
                all_consuming(tag(Type::TYPE_NAME_LONG))
            ),
            value(
                Type::ULong,
                all_consuming(tag(Type::TYPE_NAME_ULONG))
            ),
            value(
                Type::Float,
                all_consuming(tag(Type::TYPE_NAME_FLOAT))
            ),
            value(
                Type::Double,
                all_consuming(tag(Type::TYPE_NAME_DOUBLE))
            ),
            value(
                Type::Void,
                all_consuming(tag(Type::TYPE_NAME_VOID))
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
                if dna_type.size > 0 {
                    Type::Struct {
                        name: String::from(parsed_type),
                        size: dna_type.size,
                    }
                }
                else {
                    Type::Special {
                        name: String::from(parsed_type),
                    }
                }
            })
    };

    let result: IResult<&str, Type> = alt((
        parse_primitive_type,
        parse_struct_type,
    ))(name);

    match result {
        Ok((_, parsed_type)) => Ok(parsed_type),
        Err(_) => Err(AnalyseError::UnknownType { value: String::from(name) })
    }
}

pub fn analyse_field(dna_field: &DnaField, dna: &Dna, offset: usize) -> Result<Field> {
    let field_name = dna.find_field_name_of(dna_field)
        .ok_or(AnalyseError::InvalidFieldNameIndex { index: dna_field.name_index })?;
    let base_type = dna.find_type_of(dna_field)
        .ok_or(AnalyseError::InvalidTypeIndex { index: dna_field.type_index })?;

    fn parse_field_name(input: &str, base_type: &Type, dna: &Dna) -> Result<(String, Type)> {
        let result: IResult<&str, (String, Type)> = {
            all_consuming(
                map(
                    pair(
                        delimited(
                            opt(tag("(")),
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
                            opt(tag(")"))
                        ),
                        opt(value(true, tag("()")))
                    ),
                    |((pointers, name, arrays), function): ((usize, &str, Vec<&str>), Option<bool>)| {
                        let result_type = Clone::clone(base_type);
                        let result_type = if function.is_none() {
                            let result_type = if pointers > 0 {
                                (0..pointers).fold(result_type, |result, _| {
                                    Type::Pointer { base_type: Box::new(result), size: dna.pointer_size }
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
                            result_type
                        } else {
                            Type::Function { size: dna.pointer_size }
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

    analyse_type(&base_type, dna).and_then(|base_type| {
        parse_field_name(field_name, &base_type, dna)
            .map(|(parsed_name, parsed_type)| {
                Field {
                    data_type: parsed_type,
                    name: String::from(parsed_name),
                    offset
                }
            })
    })
}

pub fn analyse_struct(dna_struct: &DnaStruct, dna: &Dna) -> Result<Struct> {

    let (name, size) = dna.find_type_of(dna_struct)
        .map(|dna_type| (Clone::clone(&dna_type.name), dna_type.size))
        .ok_or(AnalyseError::InvalidTypeIndex { index: dna_struct.type_index })?;

    let fields = {
        let mut offset = 0usize;  //TODO: Remove offset. Not used anymore.
        dna_struct.fields.iter()
            .map(| dna_field| {
                let result = analyse_field(dna_field, dna, offset);
                result.iter().for_each(|field| offset = offset + field.size());
                result
            })
            .collect::<Result<Vec<Field>>>()?
    };

    Ok(Struct::new(name, fields, size))
}

#[cfg(test)]
mod test {
    use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

    use crate::analyse::{Mode};
    use crate::analyse::analysers::analyse;
    use crate::parse::{Endianness, parse};

    #[test]
    fn test_analyse() {
        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let (file_header, file_blocks, dna) = parse(blend_data.as_slice()).unwrap();

        let result = analyse(&file_header, &file_blocks, &dna, Mode::RequiredOnly).unwrap();

        assert_that!(result.endianness, is(equal_to(Endianness::Little)));
        assert_that!(result.structs.len(), is(equal_to(297)));
    }

    mod type_spec {
        use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

        use crate::analyse::{AnalyseError, Type};
        use crate::analyse::analysers::analyse_type;
        use crate::parse::{Dna, DnaStruct, DnaType};

        #[test]
        fn test_parse_type() {
            let dna = Dna {
                field_names: vec![],
                types: vec![
                    DnaType {
                        name: String::from("Material"),
                        size: 42
                    },
                    DnaType {
                        name: String::from("CustomData_MeshMasks"),
                        size: 73
                    },
                ],
                structs: vec![
                    DnaStruct {
                        type_index: 0,
                        fields: vec![]
                    },
                    DnaStruct {
                        type_index: 1,
                        fields: vec![]
                    }
                ],
                pointer_size: 4
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
                ("Material", Ok(Type::Struct { name: String::from("Material"), size: 42 })),
                ("CustomData_MeshMasks", Ok(Type::Struct { name: String::from("CustomData_MeshMasks"), size: 42 })),
                ("", Err(AnalyseError::UnknownType { value: String::from("") })),
            ];

            matrix.iter().for_each(|(name, expected)| {
                let dna_type = DnaType {
                    name: String::from(*name),
                    size: 42
                };
                assert_that!(analyse_type(&dna_type, &dna).as_ref(), is(equal_to(expected.as_ref())))
            });
        }

        #[test]
        fn test_primitive_types_size() {
            assert_that!(Type::Char.size(), is(equal_to(1)));
            assert_that!(Type::UChar.size(), is(equal_to(1)));
            assert_that!(Type::Short.size(), is(equal_to(2)));
            assert_that!(Type::UShort.size(), is(equal_to(2)));
            assert_that!(Type::Int.size(), is(equal_to(4)));
            assert_that!(Type::Long.size(), is(equal_to(4)));
            assert_that!(Type::ULong.size(), is(equal_to(4)));
            assert_that!(Type::Float.size(), is(equal_to(4)));
            assert_that!(Type::Double.size(), is(equal_to(8)));
            assert_that!(Type::Int64.size(), is(equal_to(8)));
            assert_that!(Type::UInt64.size(), is(equal_to(8)));
            assert_that!(Type::Void.size(), is(equal_to(0)));
            assert_that!(Type::Int8.size(), is(equal_to(1)));
        }
    }

    mod field_spec {
        use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

        use crate::analyse::{Field, Type};
        use crate::analyse::analysers::analyse_field;
        use crate::parse::{Dna, DnaField, DnaType};

        #[test]
        fn test_parse_primitive_field() {
            setup(|fixture|{
                {
                    let field = DnaField {
                        name_index: 0,
                        type_index: 0
                    };
                    assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                        Field {
                            data_type: Type::Float,
                            name: String::from("x"),
                            offset: 0,
                        }
                    ))));
                }
                {
                    let field = DnaField {
                        name_index: 4,
                        type_index: 0
                    };
                    assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                        Field {
                            data_type: Type::Float,
                            name: String::from("val2"),
                            offset: 0,
                        }
                    ))));
                }
                {
                    let field = DnaField {
                        name_index: 5,
                        type_index: 1
                    };
                    assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                        Field {
                            data_type: Type::Void,
                            name: String::from("_pad"),
                            offset: 0,
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
                assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                    Field {
                        data_type: Type::Struct { name: String::from("ID"), size: 42 },
                        name: String::from("id"),
                        offset: 0,
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
                    assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                        Field {
                            data_type: Type::Pointer { base_type: Box::new(Type::Void), size: 4 },
                            name: String::from("next"),
                            offset: 0,
                        }
                    ))));
                }
                {
                    let field = DnaField {
                        name_index: 9,
                        type_index: 4
                    };
                    assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                        Field {
                            data_type: Type::Pointer { base_type: Box::new(Type::Char), size: 4 },
                            name: String::from("ui_data"),
                            offset: 0,
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
                assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                    Field {
                        data_type: Type::Pointer { base_type: Box::new(Type::Pointer { base_type: Box::new(Type::Void), size: 4 }), size: 4 },
                        name: String::from("mat"),
                        offset: 0,
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
                assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                    Field {
                        data_type: Type::Pointer {
                            base_type: Box::new(
                                Type::Pointer {
                                    base_type: Box::new(
                                        Type::Struct {
                                            name: String::from("Material"),
                                            size: 73
                                        }
                                    ),
                                    size: 4
                                }
                            ),
                            size: 4
                        },
                        name: String::from("mat"),
                        offset: 0,
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
                assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                    Field {
                        data_type: Type::Array { base_type: Box::new(Type::Char), length: 1024 },
                        name: String::from("name"),
                        offset: 0,
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
                assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                    Field {
                        data_type: Type::Array { base_type: Box::new(Type::Pointer { base_type: Box::new(Type::Char), size: 4 }), length: 4 },
                        name: String::from("_pad_1"),
                        offset: 0,
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
                assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                    Field {
                        data_type: Type::Array { base_type: Box::new(Type::Array { base_type: Box::new(Type::Float), length: 3 }), length: 4 },
                        name: String::from("scale"),
                        offset: 0,
                    }
                ))));
            });
        }

        #[test]
        fn test_parse_function_field() {
            setup(|fixture| {
                let field = DnaField {
                    name_index: 12,
                    type_index: 1
                };
                assert_that!(analyse_field(&field, &fixture.dna, 0), is(equal_to(Ok(
                    Field {
                        data_type: Type::Function { size: 4 },
                        name: String::from("bind"),
                        offset: 0,
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
                        "(*bind)()",
                    ].into_iter().map(|name| String::from(name)).collect(),
                    types: vec![
                        DnaType::new("float", 4),
                        DnaType::new("void", 0),
                        DnaType::new("Material", 73),
                        DnaType::new("ID", 42),
                        DnaType::new("char", 1),
                    ],
                    structs: vec![],
                    pointer_size: 4
                }
            };

            test_code(fixture);
        }
    }

    mod struct_spec {
        use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

        use crate::analyse::{Field, Struct, Type};
        use crate::analyse::analysers::analyse_struct;
        use crate::parse::{Dna, DnaField, DnaStruct, DnaType};

        #[test]
        fn test_parse_struct() {

            let dna = Dna {
                field_names: vec![
                    "*next",
                    "name[24]",
                    "id",
                    "scale[3]",
                    "(*function)()"
                ].into_iter().map(|name| String::from(name)).collect(),
                types: vec![
                    DnaType::new("ID", 42),
                    DnaType::new("char", 1),
                    DnaType::new("Mesh", 73),
                    DnaType::new("float", 4),
                    DnaType::new("int", 4),
                ],
                structs: vec![
                    DnaStruct {
                        type_index: 3,
                        fields: vec![
                            DnaField { name_index: 0, type_index: 0 },
                            DnaField { name_index: 1, type_index: 1 },
                        ]
                    }
                ],
                pointer_size: 4
            };

            let dna_struct = DnaStruct {
                type_index: 2,
                fields: vec![
                    DnaField { name_index: 2, type_index: 0 },
                    DnaField { name_index: 3, type_index: 3 },
                    DnaField { name_index: 4, type_index: 4 },
                ]
            };

            assert_that!(analyse_struct(&dna_struct, &dna), is(equal_to(Ok(
                Struct::new(
                    String::from("Mesh"),
                    vec![
                        Field { name: String::from("id"), data_type: Type::Struct { name: String::from("ID"), size: 42 }, offset: 0 },
                        Field { name: String::from("scale"), data_type: Type::Array { base_type: Box::new(Type::Float), length: 3 }, offset: 42 },
                        Field { name: String::from("function"), data_type: Type::Function { size: 4 }, offset: 54 },
                    ],
                    73
                )
            ))));
        }
    }
}
