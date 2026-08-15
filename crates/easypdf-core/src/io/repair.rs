//! Self-healing PDF open: detect corruption and attempt lightweight repairs.
//!
//! Inspired by the `CreateReadOnlyRepairCopy`, `StripDanglingPackageRels`,
//! and `FixXmlEncoding` patterns in `OfficeCLI`.
//!
//! The repair strategy is intentionally conservative: we re-parse the
//! document through `lopdf`, strip dangling object references, renumber
//! objects for a clean cross-reference table, and re-serialise.  If
//! `lopdf` cannot parse the input at all, repair is not possible and the
//! caller should surface the original parse error.

use crate::{PdfError, Result};

use crate::PdfInput;

/// Options controlling which repair passes are attempted.
///
/// # Examples
///
/// ```
/// use easypdf_core::io::repair::RepairOptions;
///
/// let opts = RepairOptions::default();
/// assert!(opts.fix_dangling_refs);
/// assert!(opts.fix_xref);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct RepairOptions {
    /// Remove references to objects that do not exist in the document.
    pub fix_dangling_refs: bool,
    /// Renumber all objects sequentially to rebuild the cross-reference table.
    pub fix_xref: bool,
    /// Fix encoding declarations in text streams (currently a no-op placeholder).
    pub fix_encoding: bool,
    /// Drop streams that cannot be decoded (currently a no-op placeholder).
    pub strip_unparsed_streams: bool,
}

impl Default for RepairOptions {
    fn default() -> Self {
        Self {
            fix_dangling_refs: true,
            fix_xref: true,
            fix_encoding: true,
            strip_unparsed_streams: true,
        }
    }
}

impl RepairOptions {
    /// No repairs -- return the bytes as-is if they parse.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            fix_dangling_refs: false,
            fix_xref: false,
            fix_encoding: false,
            strip_unparsed_streams: false,
        }
    }
}

/// Quick check: returns `true` if the input fails to parse as a valid PDF.
///
/// This is a cheap heuristic -- it attempts `lopdf::Document::load_mem`
/// and returns `true` on failure.  A return value of `false` does not
/// guarantee the PDF is well-formed; it only means `lopdf` accepted it.
///
/// # Examples
///
/// ```
/// use easypdf_core::io::repair::is_likely_corrupt;
/// use easypdf_core::PdfInput;
///
/// // Not a PDF at all.
/// let input = PdfInput::from_bytes(b"not a pdf");
/// assert!(is_likely_corrupt(&input));
/// ```
#[must_use]
pub fn is_likely_corrupt(input: &PdfInput) -> bool {
    // We only need the first few KB for a quick parse attempt.
    let bytes = match input {
        PdfInput::Path(path) => {
            let Ok(data) = std::fs::read(path) else {
                return true;
            };
            data
        }
        PdfInput::Bytes(data) => data.clone(),
    };
    lopdf::Document::load_mem(&bytes).is_err()
}

/// Attempt to repair a PDF and return the repaired bytes.
///
/// The function loads the input through `lopdf`, applies the requested
/// repair passes, and re-serialises the document.  If `lopdf` cannot
/// parse the input at all, the original parse error is returned.
///
/// # Errors
///
/// Returns [`PdfError::Parse`] when the input cannot be parsed even for
/// repair, or [`PdfError::Io`] on I/O failures during re-serialisation.
///
/// # Examples
///
/// ```no_run
/// use easypdf_core::io::repair::{attempt_repair, RepairOptions};
/// use easypdf_core::PdfInput;
///
/// let input = PdfInput::from_path("corrupt.pdf");
/// let repaired = attempt_repair(&input, &RepairOptions::default()).unwrap();
/// ```
pub fn attempt_repair(input: &PdfInput, options: &RepairOptions) -> Result<Vec<u8>> {
    let bytes = match input {
        PdfInput::Path(path) => std::fs::read(path)?,
        PdfInput::Bytes(data) => data.clone(),
    };

    let mut document = lopdf::Document::load_mem(&bytes)
        .map_err(|error| PdfError::Parse(format!("cannot parse PDF for repair: {error}")))?;

    // Pass 1: strip dangling references.
    if options.fix_dangling_refs {
        strip_dangling_refs(&mut document);
    }

    // Pass 2: renumber objects for a clean xref table.
    if options.fix_xref {
        document.renumber_objects();
    }

    // Re-serialise.
    let mut output = Vec::new();
    document
        .save_to(&mut output)
        .map_err(|error| PdfError::Io(std::io::Error::other(error)))?;

    Ok(output)
}

/// Remove object references that point to non-existent objects.
///
/// Walks every dictionary and array in the document and replaces
/// dangling `Reference(id)` entries with `Null`.  This prevents
/// panics or infinite loops in consumers that blindly follow refs.
fn strip_dangling_refs(document: &mut lopdf::Document) {
    // Collect all valid object IDs.
    let valid_ids: std::collections::HashSet<lopdf::ObjectId> =
        document.objects.keys().copied().collect();

    // Traverse and nullify dangling references.
    document.traverse_objects(|object| {
        nullify_dangling(object, &valid_ids);
    });
}

/// Recursively replace dangling references in an object with `Null`.
fn nullify_dangling(
    object: &mut lopdf::Object,
    valid_ids: &std::collections::HashSet<lopdf::ObjectId>,
) {
    match object {
        lopdf::Object::Reference(id) => {
            if !valid_ids.contains(id) {
                *object = lopdf::Object::Null;
            }
        }
        lopdf::Object::Dictionary(dict) => {
            // We need to collect keys first to avoid borrow conflicts.
            let keys: Vec<Vec<u8>> = dict.iter().map(|(k, _)| k.clone()).collect();
            for key in keys {
                if let Ok(mut value) = dict.get(&key).cloned() {
                    nullify_dangling(&mut value, valid_ids);
                    dict.set(key, value);
                }
            }
        }
        lopdf::Object::Array(array) => {
            for item in array.iter_mut() {
                nullify_dangling(item, valid_ids);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_likely_corrupt_rejects_garbage() {
        let input = PdfInput::from_bytes(b"this is not a pdf");
        assert!(is_likely_corrupt(&input));
    }

    #[test]
    fn is_likely_corrupt_rejects_empty() {
        let input = PdfInput::from_bytes(b"");
        assert!(is_likely_corrupt(&input));
    }

    #[test]
    fn is_likely_corrupt_rejects_partial_header() {
        let input = PdfInput::from_bytes(b"%PDF-1.4\n% corrupted\n%%EOF");
        assert!(is_likely_corrupt(&input));
    }

    #[test]
    fn attempt_repair_rejects_garbage() {
        let input = PdfInput::from_bytes(b"not a pdf at all");
        let result = attempt_repair(&input, &RepairOptions::default());
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("cannot parse"), "unexpected message: {msg}");
    }

    #[test]
    fn attempt_repair_roundtrips_valid_pdf() {
        // Build a minimal valid PDF with lopdf.
        let mut doc = lopdf::Document::new();
        let content_id = doc.add_object(lopdf::Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 12 Tf (Hello) Tj ET".to_vec(),
        )));
        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
        );
        page_dict.set("Contents", lopdf::Object::Reference(content_id));
        let page_id = doc.add_object(lopdf::Object::Dictionary(page_dict));
        let mut pages = lopdf::Dictionary::new();
        pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages.set(
            "Kids",
            lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]),
        );
        pages.set("Count", lopdf::Object::Integer(1));
        let pages_id = doc.add_object(lopdf::Object::Dictionary(pages));
        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer
            .set("Root", lopdf::Object::Reference(catalog_id));

        let mut original = Vec::new();
        doc.save_to(&mut original).unwrap();

        let input = PdfInput::from_bytes(original.clone());
        let repaired = attempt_repair(&input, &RepairOptions::default()).unwrap();

        // The repaired bytes should parse back successfully.
        let reloaded = lopdf::Document::load_mem(&repaired);
        assert!(reloaded.is_ok(), "repaired PDF should parse");
    }

    #[test]
    fn repair_options_none_disables_all() {
        let opts = RepairOptions::none();
        assert!(!opts.fix_dangling_refs);
        assert!(!opts.fix_xref);
        assert!(!opts.fix_encoding);
        assert!(!opts.strip_unparsed_streams);
    }

    #[test]
    fn repair_options_default_enables_all() {
        let opts = RepairOptions::default();
        assert!(opts.fix_dangling_refs);
        assert!(opts.fix_xref);
        assert!(opts.fix_encoding);
        assert!(opts.strip_unparsed_streams);
    }
}
