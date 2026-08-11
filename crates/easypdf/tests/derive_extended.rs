//! Integration test: extended `#[derive(PdfModel)]` attributes.
#![allow(dead_code)]

use easypdf::prelude::*;

// --- Basic field attribute ---

#[derive(PdfModel)]
#[pdf(page = PageSize::A4, orientation = Orientation::Portrait)]
struct BasicModel {
    #[pdf(text, position = (72.0, 700.0))]
    title: String,
}

#[test]
fn basic_model_renders() {
    let model = BasicModel {
        title: "Hello".to_string(),
    };
    let elements = model.render().unwrap();
    assert_eq!(elements.len(), 1);
    let meta = model.metadata();
    assert_eq!(meta.page_size, PageSize::A4);
    assert_eq!(meta.orientation, Orientation::Portrait);
}

// --- field attribute ---

#[derive(PdfModel)]
struct FieldModel {
    #[pdf(field = "customer_name", text, position = (72.0, 700.0))]
    customer: String,
}

#[test]
fn field_attribute_generates_descriptor() {
    let model = FieldModel {
        customer: "Alice".to_string(),
    };
    let descriptors = model.field_descriptors();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].field_name, "customer_name");
    assert_eq!(descriptors[0].rust_field_name, "customer");
}

// --- order attribute ---

#[derive(PdfModel)]
struct OrderModel {
    #[pdf(field = "second", order = 2, text, position = (72.0, 600.0))]
    b: String,
    #[pdf(field = "first", order = 1, text, position = (72.0, 700.0))]
    a: String,
}

#[test]
fn order_attribute_recorded() {
    let model = OrderModel {
        a: "A".to_string(),
        b: "B".to_string(),
    };
    let mut descriptors = model.field_descriptors();
    descriptors.sort_by_key(|d| d.order);
    assert_eq!(descriptors[0].field_name, "first");
    assert_eq!(descriptors[0].order, 1);
    assert_eq!(descriptors[1].field_name, "second");
    assert_eq!(descriptors[1].order, 2);
}

// --- skip / ignore attribute ---

#[derive(PdfModel)]
struct SkipModel {
    #[pdf(text, position = (72.0, 700.0))]
    visible: String,
    #[pdf(skip)]
    internal_id: u64,
}

#[test]
fn skip_attribute_excludes_from_render() {
    let model = SkipModel {
        visible: "yes".to_string(),
        internal_id: 42,
    };
    let elements = model.render().unwrap();
    assert_eq!(elements.len(), 1); // only visible field
    let descriptors = model.field_descriptors();
    // skip fields with no form attributes are excluded from descriptors
    assert!(descriptors.is_empty());
}

#[derive(PdfModel)]
struct IgnoreModel {
    #[pdf(text, position = (72.0, 700.0))]
    name: String,
    #[pdf(ignore)]
    secret: String,
}

#[test]
fn ignore_attribute_excludes_from_render() {
    let model = IgnoreModel {
        name: "test".to_string(),
        secret: "hidden".to_string(),
    };
    let elements = model.render().unwrap();
    assert_eq!(elements.len(), 1);
}

// --- default attribute ---

#[derive(PdfModel)]
struct DefaultModel {
    #[pdf(field = "notes", default = "N/A")]
    notes: String,
}

#[test]
fn default_attribute_recorded() {
    let model = DefaultModel {
        notes: String::new(),
    };
    let descriptors = model.field_descriptors();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].default_value.as_deref(), Some("N/A"));
}

// --- required attribute ---

#[derive(PdfModel)]
struct RequiredModel {
    #[pdf(field = "email", required)]
    email: String,
}

#[test]
fn required_attribute_recorded() {
    let model = RequiredModel {
        email: "test@example.com".to_string(),
    };
    let descriptors = model.field_descriptors();
    assert_eq!(descriptors.len(), 1);
    assert!(descriptors[0].required);
}

// --- format attribute ---

#[derive(PdfModel)]
struct FormatModel {
    #[pdf(field = "date", format = "YYYY-MM-DD")]
    date: String,
}

#[test]
fn format_attribute_recorded() {
    let model = FormatModel {
        date: "2026-08-10".to_string(),
    };
    let descriptors = model.field_descriptors();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].format.as_deref(), Some("YYYY-MM-DD"));
}

// --- nested attribute ---

#[derive(PdfModel)]
struct Address {
    #[pdf(text, position = (72.0, 650.0))]
    city: String,
}

#[derive(PdfModel)]
struct NestedModel {
    #[pdf(text, position = (72.0, 700.0))]
    name: String,
    #[pdf(nested)]
    address: Address,
}

#[test]
fn nested_attribute_renders_inner_elements() {
    let model = NestedModel {
        name: "Alice".to_string(),
        address: Address {
            city: "Beijing".to_string(),
        },
    };
    let elements = model.render().unwrap();
    assert_eq!(elements.len(), 2); // name + address.city

    let descriptors = model.field_descriptors();
    let addr_desc = descriptors.iter().find(|d| d.field_name == "address").unwrap();
    assert!(addr_desc.nested);
}

// --- Combined attributes ---

#[derive(PdfModel)]
struct Invoice {
    #[pdf(field = "customer_name", order = 1, required)]
    customer: String,
    #[pdf(field = "invoice_date", order = 2, format = "YYYY-MM-DD")]
    date: String,
    #[pdf(skip)]
    internal_id: u64,
    #[pdf(field = "notes", order = 4, default = "none")]
    notes: String,
}

#[test]
fn combined_attributes() {
    let model = Invoice {
        customer: "Bob".to_string(),
        date: "2026-01-15".to_string(),
        internal_id: 12345,
        notes: String::new(),
    };

    let elements = model.render().unwrap();
    // No text/table/image attributes, so no rendered elements
    assert!(elements.is_empty());

    let mut descriptors = model.field_descriptors();
    // internal_id is skipped, so only 3 descriptors
    assert_eq!(descriptors.len(), 3);

    descriptors.sort_by_key(|d| d.order);
    assert_eq!(descriptors[0].field_name, "customer_name");
    assert!(descriptors[0].required);
    assert_eq!(descriptors[1].field_name, "invoice_date");
    assert_eq!(descriptors[1].format.as_deref(), Some("YYYY-MM-DD"));
    assert_eq!(descriptors[2].field_name, "notes");
    assert_eq!(descriptors[2].default_value.as_deref(), Some("none"));
}

// --- Metadata ---

#[test]
fn struct_level_metadata() {
    #[derive(PdfModel)]
    #[pdf(page = PageSize::Letter, orientation = Orientation::Landscape, margins = 36.0)]
    struct CustomMeta {
        #[pdf(text, position = (0.0, 0.0))]
        x: String,
    }

    let model = CustomMeta { x: String::new() };
    let meta = model.metadata();
    assert_eq!(meta.page_size, PageSize::Letter);
    assert_eq!(meta.orientation, Orientation::Landscape);
    assert!((meta.margins - 36.0).abs() < f64::EPSILON);
}
