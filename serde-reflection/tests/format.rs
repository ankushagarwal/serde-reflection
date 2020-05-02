// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_reflection::{ContainerFormat, Error, Format, FormatHolder, Named, VariantFormat};
use std::collections::HashSet;

#[test]
fn test_format_visiting() {
    use Format::*;

    let format = ContainerFormat::Enum(
        vec![(
            0,
            Named {
                name: "foo".into(),
                value: VariantFormat::Tuple(vec![
                    TypeName("foo".into()),
                    TypeName("bar".into()),
                    Seq(Box::new(TypeName("foo".into()))),
                ]),
            },
        )]
        .into_iter()
        .collect(),
    );
    let mut names = HashSet::new();
    format
        .visit(&mut |f| {
            if let TypeName(x) = f {
                // Insert a &str borrowed from `format`.
                names.insert(x.as_str());
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(names.len(), 2);

    assert!(VariantFormat::unknown().visit(&mut |_| Ok(())).is_err());
    assert!(Format::unknown().visit(&mut |_| Ok(())).is_err());
}

#[test]
fn test_format_unification() {
    use Format::*;

    let mut x = Format::unknown();
    assert!(x.unify(U8).is_ok());
    x.reduce();
    assert_eq!(x, U8);
    assert_eq!(
        x.unify(U16).unwrap_err(),
        Error::Incompatible("U8".into(), "U16".into())
    );

    let mut x = Tuple(vec![Format::unknown(), U32]);
    x.unify(Tuple(vec![U16, Format::unknown()])).unwrap();
    x.reduce();
    assert_eq!(x, Tuple(vec![U16, U32]));

    for x in vec![
        Unit,
        Bool,
        I8,
        I16,
        I32,
        I64,
        I128,
        U8,
        U16,
        U32,
        U64,
        U128,
        F32,
        F64,
        Char,
        Str,
        Bytes,
        TypeName("foo".into()),
        Option(Box::new(Unit)),
        Seq(Box::new(Unit)),
        Map {
            key: Box::new(Unit),
            value: Box::new(Unit),
        },
        Tuple(vec![Format::unknown()]),
    ]
    .iter_mut()
    {
        assert!(x.unify(TypeName("bar".into())).is_err());
        assert!(x.unify(Option(Box::new(U32))).is_err());
        assert!(x.unify(Seq(Box::new(U32))).is_err());
        assert!(x.unify(Tuple(vec![])).is_err());
    }
}

#[test]
fn test_container_format_unification() {
    use ContainerFormat::*;
    use Format::*;

    let mut x = TupleStruct(vec![Format::unknown(), U32]);
    x.unify(TupleStruct(vec![U16, Format::unknown()])).unwrap();
    x.reduce();
    assert_eq!(x, TupleStruct(vec![U16, U32]));

    let mut x = Enum(
        vec![(
            0,
            Named {
                name: "foo".into(),
                value: VariantFormat::Tuple(vec![Format::unknown()]),
            },
        )]
        .into_iter()
        .collect(),
    );
    assert!(x
        .unify(Enum(
            vec![(
                0,
                Named {
                    name: "foo".into(),
                    value: VariantFormat::Unit,
                }
            )]
            .into_iter()
            .collect()
        ))
        .is_err());
    assert!(x
        .unify(Enum(
            vec![(
                0,
                Named {
                    name: "foo".into(),
                    value: VariantFormat::Tuple(vec![U8]),
                }
            )]
            .into_iter()
            .collect()
        ))
        .is_ok());

    for x in vec![
        UnitStruct,
        NewTypeStruct(Box::new(Unit)),
        TupleStruct(vec![Format::unknown()]),
        Struct(vec![Named {
            name: "foo".into(),
            value: Format::unknown(),
        }]),
        Enum(
            vec![(
                0,
                Named {
                    name: "foo".into(),
                    value: VariantFormat::unknown(),
                },
            )]
            .into_iter()
            .collect(),
        ),
    ]
    .iter_mut()
    {
        assert!(x.unify(NewTypeStruct(Box::new(U8))).is_err());
        assert!(x.unify(TupleStruct(vec![])).is_err());
        assert!(x
            .unify(Struct(vec![Named {
                name: "bar".into(),
                value: Format::unknown()
            }]))
            .is_err());
        assert!(x
            .unify(Enum(
                vec![(
                    0,
                    Named {
                        name: "bar".into(),
                        value: VariantFormat::unknown()
                    }
                )]
                .into_iter()
                .collect()
            ))
            .is_err());
    }
}
