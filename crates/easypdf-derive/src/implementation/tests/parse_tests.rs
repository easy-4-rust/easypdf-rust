use super::*;

// ---- core_crate() ----

#[test]
fn core_crate_returns_ident() {
    let ts = core_crate();
    let s = ts.to_string();
    // In test context, proc_macro_crate can't find the crate, so fallback
    assert_eq!(s, "easypdf_core");
}

// ---- get_named_fields() ----

#[test]
fn get_named_fields_rejects_enum() {
    let input = parse_derive_input("enum Foo { Bar }");
    let result = get_named_fields(&input);
    assert!(result.is_err());
}

#[test]
fn get_named_fields_rejects_tuple_struct() {
    let input = parse_derive_input("struct Foo(String);");
    let result = get_named_fields(&input);
    assert!(result.is_err());
}

#[test]
fn get_named_fields_accepts_named_struct() {
    let input = parse_derive_input("struct Foo { x: i32 }");
    let result = get_named_fields(&input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

// ---- parse_struct_attrs() ----

#[test]
fn parse_struct_attrs_defaults() {
    let input = parse_derive_input("struct Foo { x: i32 }");
    let result = parse_struct_attrs(&input.attrs).unwrap();
    // Check defaults compile (we can't easily compare TokenStreams for equality)
    assert!(!result.page_size.is_empty());
    assert!(!result.orientation.is_empty());
    assert!(!result.margins.is_empty());
}

#[test]
fn parse_struct_attrs_with_page() {
    let input = parse_derive_input(
        r#"#[pdf(page = easypdf_core::PageSize::A3)]
        struct Foo { x: i32 }"#,
    );
    let result = parse_struct_attrs(&input.attrs).unwrap();
    assert!(!result.page_size.is_empty());
}

#[test]
fn parse_struct_attrs_with_orientation() {
    let input = parse_derive_input(
        r#"#[pdf(orientation = easypdf_core::Orientation::Landscape)]
        struct Foo { x: i32 }"#,
    );
    let result = parse_struct_attrs(&input.attrs).unwrap();
    assert!(!result.orientation.is_empty());
}

#[test]
fn parse_struct_attrs_with_margins() {
    let input = parse_derive_input(r#"#[pdf(margins = 36.0_f64)] struct Foo { x: i32 }"#);
    let result = parse_struct_attrs(&input.attrs).unwrap();
    assert!(!result.margins.is_empty());
}

#[test]
fn parse_struct_attrs_ignores_non_pdf_attrs() {
    let input = parse_derive_input(
        r#"#[derive(Debug)]
        struct Foo { x: i32 }"#,
    );
    let result = parse_struct_attrs(&input.attrs).unwrap();
    assert!(!result.page_size.is_empty());
}

// ---- parse_field_attrs() ----

#[test]
fn parse_field_no_attrs() {
    let input = parse_derive_input("struct Foo { x: i32 }");
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(!pf.skip);
    assert!(pf.field_name.is_none());
    assert!(pf.position.is_none());
    assert!(pf.order.is_none());
    assert!(pf.default_value.is_none());
    assert!(!pf.required);
    assert!(pf.format.is_none());
    assert!(!pf.nested);
    assert!(matches!(pf.render_kind, RenderKind::None));
}

#[test]
fn parse_field_text_with_position() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(text, position = (100, 700))]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(matches!(pf.render_kind, RenderKind::Text));
    assert!(pf.position.is_some());
}

#[test]
fn parse_field_table() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(table, position = (72, 500))]
            data: Vec<Vec<String>>,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(matches!(pf.render_kind, RenderKind::Table));
}

#[test]
fn parse_field_image() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(image, position = (72, 500))]
            logo: Vec<u8>,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(matches!(pf.render_kind, RenderKind::Image));
}

#[test]
fn parse_field_ignore() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(ignore)]
            internal: i32,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(pf.skip);
}

#[test]
fn parse_field_skip_alias() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(skip)]
            internal: i32,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(pf.skip);
}

#[test]
fn parse_field_field_name() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(field = "pdf_title")]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert_eq!(pf.field_name.as_deref(), Some("pdf_title"));
}

#[test]
fn parse_field_order() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(order = 5)]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert_eq!(pf.order, Some(5));
}

#[test]
fn parse_field_default() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(default = "N/A")]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert_eq!(pf.default_value.as_deref(), Some("N/A"));
}

#[test]
fn parse_field_required() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(required)]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(pf.required);
}

#[test]
fn parse_field_format() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(format = "YYYY-MM-DD")]
            date: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert_eq!(pf.format.as_deref(), Some("YYYY-MM-DD"));
}

#[test]
fn parse_field_nested() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(nested)]
            inner: Bar,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(pf.nested);
}

#[test]
fn parse_field_text_with_size() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(text, size = 14)]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let pf = parse_field_attrs(&fields[0]).unwrap();
    assert!(matches!(pf.render_kind, RenderKind::Text));
    assert!(!pf.text_attrs.is_empty());
}

// ---- Validation errors ----

#[test]
fn skip_and_field_mutually_exclusive() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(skip, field = "x")]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let result = parse_field_attrs(&fields[0]);
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("skipped"));
    }
}

#[test]
fn nested_with_text_rejected() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(nested, text)]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let result = parse_field_attrs(&fields[0]);
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("nested"));
    }
}

#[test]
fn nested_with_table_rejected() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(nested, table)]
            data: Vec<String>,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let result = parse_field_attrs(&fields[0]);
    assert!(result.is_err());
}

#[test]
fn nested_with_image_rejected() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(nested, image)]
            img: Vec<u8>,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let result = parse_field_attrs(&fields[0]);
    assert!(result.is_err());
}

#[test]
fn unknown_attribute_rejected() {
    let input = parse_derive_input(
        r#"struct Foo {
            #[pdf(bogus = "value")]
            title: String,
        }"#,
    );
    let fields = get_named_fields(&input).unwrap();
    let result = parse_field_attrs(&fields[0]);
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("unknown pdf attribute"));
    }
}
