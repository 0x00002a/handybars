# Handybars

## Introduction

This is a small library for template expansion. The syntax is based on
handlebars, but it _only_ support expansion of variables. No `#if` or `#each`,
only `{{ variable }}`. If you need actual handlebars support consider the
[handlebars](https://lib.rs/crates/handlebars) crate.

It has no dependencies and is designed to have a very simple API.

A simple [companion attribute macro](attribute/README.md) allows you to automatically turn enums and
structs into Handybar [Value](https://docs.rs/handybars/latest/handybars/enum.Value.html) variants.

## Basic Usage

```rust
use handybars::{Context, Variable};
let ctx = Context::new().with_define("hello".parse().unwrap(), "world");
assert_eq!(ctx.render("hello {{ hello }}"), Ok("hello world".to_owned()));
```

You can also define objects

```rust
# use handybars::{Context, Variable, Object};
# let mut ctx = Context::new().with_define("hello".parse().unwrap(), "world");
ctx.define("obj".parse().unwrap(), Object::new().with_property("a", "value"));
assert_eq!(ctx.render("object a: {{ obj.a }}"), Ok("object a: value".to_owned()));
```

You can even have nested objects

```rust
# use handybars::{Context, Variable, Object};
# let mut ctx = Context::new().with_define("hello".parse().unwrap(), "world");
ctx.define("obj".parse().unwrap(), Object::new().with_property("b", Object::new().with_property("c", "value")));
assert_eq!(ctx.render("nested: {{ obj.b.c }}"), Ok("nested: value".to_owned()));
```

Note that objects cannot be directly expanded:

```rust
use handybars::{Context, Variable, Object, Error};
let ctx = Context::new().with_define("world".parse().unwrap(), Object::new().with_property("a", "p1"));
assert_eq!(ctx.render("{{world}}"), Err(Error::TriedToExpandObject(Variable::single("world"))));
```

## Macros Usage

Usage of these requires the `macros` feature.

Enums are converted to [Value::String](https://docs.rs/handybars/latest/handybars/enum.Value.html) variants:

```rust
# #[cfg(feature = "macros")]
use handybars::handybars_value;
# #[cfg(feature = "macros")]
#[handybars_value]
enum SimpleEnumProp {
    A,
    B,
}
```

Use structs as [Value::Object](https://docs.rs/handybars/latest/handybars/enum.Value.html) variants::

```rust
# #[cfg(feature = "macros")]
# use handybars::handybars_value;
# #[handybars_value]
# #[cfg(feature = "macros")]
# enum SimpleEnumProp {
#     A,
#     B,
# }
# #[cfg(feature = "macros")]
#[handybars_value]
struct StructVal<'a> {
    field_1: u16,
    field_2: String,
    field_3: &'a str,
    field_4: SimpleEnumProp,
}
```

Combine enums and structs into more complex objects:

```rust
# #[cfg(feature = "macros")]
# use handybars::handybars_value;
# #[cfg(feature = "macros")]
# #[handybars_value]
# enum SimpleEnumProp {
#     A,
#     B,
# }
# #[cfg(feature = "macros")]
# #[handybars_value]
# struct StructVal<'a> {
#     field_1: u16,
#     field_2: String,
#     field_3: &'a str,
#     field_4: SimpleEnumProp,
# }
# #[cfg(feature = "macros")]
#[handybars_value]
struct TestObject<'a> {
    prop_0: String,
    prop_1: u64,
    prop_2: &'a str,
    prop_3: StructVal<'a>,
    prop_4: SimpleEnumProp,
}
```

It also works on nested structs:

```rust
# #[cfg(feature = "macros")]
# use handybars::{ Context, Variable, handybars_value};
# #[cfg(feature = "macros")]
# #[handybars_value]
# enum SimpleEnumProp {
#     A,
#     B,
# }
# #[cfg(feature = "macros")]
# #[handybars_value]
# struct StructVal<'a> {
#     field_1: u16,
#     field_2: String,
#     field_3: &'a str,
#     field_4: SimpleEnumProp,
# }
# #[cfg(feature = "macros")]
# #[handybars_value]
# struct TestObject<'a> {
#     prop_0: String,
#     prop_1: u64,
#     prop_2: &'a str,
#     prop_3: StructVal<'a>,
#     prop_4: SimpleEnumProp,
# }
# #[cfg(feature = "macros")]
let v = TestObject {
    prop_0: "p0_val".to_owned(),
    prop_1: 1,
    prop_2: "p2_val",
    prop_3: StructVal {
        field_1: 30,
        field_2: "f32_val".to_owned(),
        field_3: "f33_val",
        field_4: SimpleEnumProp::A,
    },
    prop_4: SimpleEnumProp::B,
};
# #[cfg(feature = "macros")]
let c = Context::new().with_define(Variable::single("obj"), v);
# #[cfg(feature = "macros")]
assert_eq!("1", c.render("{{ obj.prop_1 }}").unwrap());
# #[cfg(feature = "macros")]
assert_eq!("A", c.render("{{ obj.prop_3.field_4 }}").unwrap());
# #[cfg(feature = "macros")]
assert_eq!("f33_val", c.render("{{ obj.prop_3.field_3 }}").unwrap());
```

Enums with variant values are currently **not supported**. Enum with variants like the following **will not compile**:

```compile_fail
#[handybars_value]
enum ComplexEnumProp<'a> {
    Var1(SimpleEnumProp),
    Var2(String),
    Var3(StructVal<'a>),
}
```
