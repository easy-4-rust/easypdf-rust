//! 自修复 PDF 打开：检测损坏并尝试轻量级修复。
//!
//! 灵感来自 `OfficeCLI` 中的 `CreateReadOnlyRepairCopy`、
//! `StripDanglingPackageRels` 和 `FixXmlEncoding` 模式。
//!
//! 修复策略有意保守：通过 `lopdf` 重新解析文档，剥离悬挂的
//! 对象引用，重新编号对象以获得干净的交叉引用表，然后重新序列化。
//! 如果 `lopdf` 完全无法解析输入，则无法修复，调用方应返回
//! 原始解析错误。

use crate::{PdfError, Result};

use crate::PdfInput;

/// 控制尝试哪些修复通道的选项。
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
    /// 移除指向文档中不存在的对象的引用。
    pub fix_dangling_refs: bool,
    /// 按顺序重新编号所有对象以重建交叉引用表。
    pub fix_xref: bool,
    /// 修复文本流中的编码声明（当前为空操作占位符）。
    pub fix_encoding: bool,
    /// 丢弃无法解码的流（当前为空操作占位符）。
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
    /// 不修复——如果可解析则原样返回字节。
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

/// 快速检查：输入无法作为有效 PDF 解析时返回 `true`。
///
/// 这是廉价的启发式检查——尝试 `lopdf::Document::load_mem`
/// 失败时返回 `true`。返回 `false` 并不保证 PDF 格式正确；
/// 只意味着 `lopdf` 接受了它。
///
/// # Examples
///
/// ```
/// use easypdf_core::io::repair::is_likely_corrupt;
/// use easypdf_core::PdfInput;
///
/// // 完全不是 PDF。
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

/// 尝试修复 PDF 并返回修复后的字节。
///
/// 函数通过 `lopdf` 加载输入，应用请求的修复通道，
/// 然后重新序列化文档。如果 `lopdf` 完全无法解析输入，
/// 则返回原始解析错误。
///
/// # Errors
///
/// 输入无法解析（即使是用于修复）时返回 [`PdfError::Parse`]，
/// 或重新序列化期间的 I/O 失败返回 [`PdfError::Io`]。
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
