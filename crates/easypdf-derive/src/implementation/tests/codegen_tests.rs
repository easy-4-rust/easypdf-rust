use super::*;

// ---- expand_pdf_model() ----

#[test]
fn expand_simple_text_struct() {
    let input: TokenStream = quote! {
        struct Invoice {
            #[pdf(text, position = (100, 700))]
            title: String,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
    let expanded = result.unwrap().to_string();
    assert!(expanded.contains("impl"));
    assert!(expanded.contains("PdfModel"));
    assert!(expanded.contains("render"));
    assert!(expanded.contains("metadata"));
    assert!(expanded.contains("field_descriptors"));
}

#[test]
fn expand_struct_with_ignore_field() {
    let input: TokenStream = quote! {
        struct Invoice {
            #[pdf(text, position = (100, 700))]
            title: String,
            #[pdf(ignore)]
            internal: i32,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_struct_with_table_field() {
    let input: TokenStream = quote! {
        struct Report {
            #[pdf(table, position = (72, 500))]
            data: Vec<Vec<String>>,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_struct_with_image_field() {
    let input: TokenStream = quote! {
        struct Doc {
            #[pdf(image, position = (72, 500))]
            logo: Vec<u8>,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_struct_with_nested_field() {
    let input: TokenStream = quote! {
        struct Outer {
            #[pdf(nested)]
            inner: Inner,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_struct_with_field_descriptor() {
    let input: TokenStream = quote! {
        struct Form {
            #[pdf(field = "name", required, default = "unknown", order = 1, format = "text")]
            name: String,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
    let expanded = result.unwrap().to_string();
    assert!(expanded.contains("field_descriptors"));
}

#[test]
fn expand_struct_with_text_no_position() {
    // Text field without explicit position uses default
    let input: TokenStream = quote! {
        struct Doc {
            #[pdf(text)]
            title: String,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_struct_with_table_no_position() {
    let input: TokenStream = quote! {
        struct Doc {
            #[pdf(table)]
            data: Vec<Vec<String>>,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_struct_with_image_no_position() {
    let input: TokenStream = quote! {
        struct Doc {
            #[pdf(image)]
            img: Vec<u8>,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_struct_with_struct_attrs() {
    let input: TokenStream = quote! {
        #[pdf(page = easypdf_core::PageSize::A3, orientation = easypdf_core::Orientation::Landscape, margins = 36.0)]
        struct Doc {
            #[pdf(text, position = (100, 700))]
            title: String,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_enum_fails() {
    let input: TokenStream = quote! {
        enum Foo { Bar }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_err());
}

#[test]
fn expand_tuple_struct_fails() {
    let input: TokenStream = quote! {
        struct Foo(String);
    };
    let result = expand_pdf_model(input);
    assert!(result.is_err());
}

#[test]
fn expand_empty_struct() {
    let input: TokenStream = quote! {
        struct Empty {}
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

#[test]
fn expand_multiple_fields_mixed() {
    let input: TokenStream = quote! {
        struct Invoice {
            #[pdf(text, position = (100, 700))]
            title: String,
            #[pdf(table, position = (72, 500))]
            items: Vec<Vec<String>>,
            #[pdf(ignore)]
            internal: i32,
            #[pdf(field = "total", required)]
            total: String,
            #[pdf(image, position = (400, 100))]
            qr: Vec<u8>,
        }
    };
    let result = expand_pdf_model(input);
    assert!(result.is_ok());
}

// ---- generate_render_arms() ----

#[test]
fn render_arms_skip_field() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
        ty: syn::parse_str("i32").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: true,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_render_arms(&fields, &core);
    assert!(result.is_ok());
    // Should produce empty output since field is skipped
    assert!(result.unwrap().is_empty());
}

#[test]
fn render_arms_text_field_default_position() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("title", proc_macro2::Span::call_site()),
        ty: syn::parse_str("String").unwrap(),
        render_kind: RenderKind::Text,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_render_arms(&fields, &core).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn render_arms_table_field_default_position() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("data", proc_macro2::Span::call_site()),
        ty: syn::parse_str("Vec<Vec<String>>").unwrap(),
        render_kind: RenderKind::Table,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_render_arms(&fields, &core).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn render_arms_image_field_default_position() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("img", proc_macro2::Span::call_site()),
        ty: syn::parse_str("Vec<u8>").unwrap(),
        render_kind: RenderKind::Image,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_render_arms(&fields, &core).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn render_arms_nested_field() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("inner", proc_macro2::Span::call_site()),
        ty: syn::parse_str("Inner").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: true,
    }];
    let core = core_crate();
    let result = generate_render_arms(&fields, &core).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn render_arms_none_kind_no_nested() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
        ty: syn::parse_str("i32").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: Some("x_field".to_string()),
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_render_arms(&fields, &core).unwrap();
    // None kind without nested produces no render code
    assert!(result.is_empty());
}

// ---- generate_field_descriptors() ----

#[test]
fn descriptors_skip_no_field_name() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
        ty: syn::parse_str("i32").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: true,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(result.is_empty());
}

#[test]
fn descriptors_skip_with_field_name() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
        ty: syn::parse_str("i32").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: Some("pdf_x".to_string()),
        order: None,
        skip: true,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(!result.is_empty());
}

#[test]
fn descriptors_required_field() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("name", proc_macro2::Span::call_site()),
        ty: syn::parse_str("String").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: None,
        required: true,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(!result.is_empty());
}

#[test]
fn descriptors_field_with_default() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("name", proc_macro2::Span::call_site()),
        ty: syn::parse_str("String").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: Some("N/A".to_string()),
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(!result.is_empty());
}

#[test]
fn descriptors_nested_field() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("inner", proc_macro2::Span::call_site()),
        ty: syn::parse_str("Inner").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: true,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(!result.is_empty());
}

#[test]
fn descriptors_with_order() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("title", proc_macro2::Span::call_site()),
        ty: syn::parse_str("String").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: Some("pdf_title".to_string()),
        order: Some(3),
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(!result.is_empty());
}

#[test]
fn descriptors_with_format() {
    let fields = vec![ParsedField {
        ident: syn::Ident::new("date", proc_macro2::Span::call_site()),
        ty: syn::parse_str("String").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: Some("pdf_date".to_string()),
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: Some("YYYY-MM-DD".to_string()),
        nested: false,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(!result.is_empty());
}

#[test]
fn descriptors_no_attrs_skipped() {
    // Field with no form-related attributes should be skipped
    let fields = vec![ParsedField {
        ident: syn::Ident::new("x", proc_macro2::Span::call_site()),
        ty: syn::parse_str("i32").unwrap(),
        render_kind: RenderKind::None,
        position: None,
        text_attrs: TokenStream::new(),
        field_name: None,
        order: None,
        skip: false,
        default_value: None,
        required: false,
        format: None,
        nested: false,
    }];
    let core = core_crate();
    let result = generate_field_descriptors(&fields, &core);
    assert!(result.is_empty());
}
