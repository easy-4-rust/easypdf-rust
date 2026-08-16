//! 基于 printpdf 的写入引擎实现。
//!
//! 提供 [`PrintpdfEngine`]，实现 [`WriteEngine`](super::write_engine::WriteEngine) trait，
//! 将 [`WriterOp`] 中间表示转换为 `printpdf` 操作并生成最终 PDF。

use std::collections::HashMap;

use easypdf_core::PdfMetadata;
use easypdf_core::error::{PdfError, Result};
use printpdf::{
    BuiltinFont, ExternalXObject, FontId, Line, LinePoint, Mm, Op, PdfDocument, PdfFontHandle,
    PdfPage, PdfSaveOptions, Point, Pt, RawImage, Svg, XObjectTransform,
};

use super::op::{
    BuiltinFontKind, FontKey, LineData, LinePointData, PendingXObject, WriterOp,
    XObjectTransformData,
};
use super::write_engine::WriteEngine;

/// 基于 `printpdf` 的 PDF 写入引擎。
///
/// 内部持有 `printpdf::PdfDocument` 和自定义字体表，
/// 负责将 [`WriterOp`] 转换为 `printpdf::Op` 并生成最终 PDF。
pub(crate) struct PrintpdfEngine {
    /// 底层 printpdf 文档。
    doc: PdfDocument,
    /// 自定义字体映射（用户键名 -> printpdf FontId）。
    custom_fonts: HashMap<String, FontId>,
    /// 已构建的页面列表。
    pages: Vec<PdfPage>,
}

/// 单位转换常量：PDF 点 -> 毫米。
const PT_TO_MM: f64 = 25.4 / 72.0;

impl PrintpdfEngine {
    /// 创建新的 printpdf 引擎。
    ///
    /// # 参数
    ///
    /// - `title`：PDF 文档标题。
    pub fn new(title: &str) -> Self {
        Self {
            doc: PdfDocument::new(title),
            custom_fonts: HashMap::new(),
            pages: Vec::new(),
        }
    }
}

impl WriteEngine for PrintpdfEngine {
    fn register_font(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let mut warnings = Vec::new();
        let parsed = ParsedFont::from_bytes(data, 0, &mut warnings)
            .ok_or_else(|| PdfError::Parse(format!("Failed to parse font: {key}")))?;
        let font_id = self.doc.add_font(&parsed);
        self.custom_fonts.insert(key.to_string(), font_id);
        Ok(())
    }

    fn register_xobject(&mut self, xobject: PendingXObject) -> Result<String> {
        let xobject_id = match xobject {
            PendingXObject::Image(data) => {
                let mut warnings = Vec::new();
                let raw = RawImage::decode_from_bytes(&data, &mut warnings)
                    .map_err(|e| PdfError::Parse(format!("Image decode error: {e}")))?;
                self.doc.add_image(&raw).0
            }
            PendingXObject::Svg(svg_data) => {
                let mut warnings = Vec::new();
                let xobj: ExternalXObject = Svg::parse(&svg_data, &mut warnings)
                    .map_err(|e| PdfError::Parse(format!("SVG parse error: {e}")))?;
                self.doc.add_xobject(&xobj).0
            }
        };
        Ok(xobject_id)
    }

    fn add_page(&mut self, width_pt: f64, height_pt: f64, ops: Vec<WriterOp>) {
        let printpdf_ops = to_printpdf_ops(&ops, &self.custom_fonts);
        self.pages.push(PdfPage::new(
            Mm(width_pt as f32 * PT_TO_MM as f32),
            Mm(height_pt as f32 * PT_TO_MM as f32),
            printpdf_ops,
        ));
    }

    fn finish(&mut self, metadata: &PdfMetadata) -> Result<Vec<u8>> {
        let pages = std::mem::take(&mut self.pages);
        self.doc.with_pages(pages);

        // 应用文档元数据。
        let info = &mut self.doc.metadata.info;
        if let Some(ref title) = metadata.title {
            info.document_title.clone_from(title);
        }
        if let Some(ref author) = metadata.author {
            info.author.clone_from(author);
        }
        if let Some(ref subject) = metadata.subject {
            info.subject.clone_from(subject);
        }
        if let Some(ref keywords) = metadata.keywords {
            info.keywords = keywords.split(',').map(|s| s.trim().to_string()).collect();
        }
        if let Some(ref creator) = metadata.creator {
            info.creator.clone_from(creator);
        }
        if let Some(ref producer) = metadata.producer {
            info.producer.clone_from(producer);
        }

        let opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        Ok(self.doc.save(&opts, &mut warnings))
    }
}

// ---------------------------------------------------------------------------
// WriterOp -> printpdf::Op 转换
// ---------------------------------------------------------------------------

/// 将 [`BuiltinFontKind`] 转换为 `printpdf::BuiltinFont`。
fn to_printpdf_builtin(kind: BuiltinFontKind) -> BuiltinFont {
    match kind {
        BuiltinFontKind::TimesRoman => BuiltinFont::TimesRoman,
        BuiltinFontKind::TimesBold => BuiltinFont::TimesBold,
        BuiltinFontKind::TimesItalic => BuiltinFont::TimesItalic,
        BuiltinFontKind::TimesBoldItalic => BuiltinFont::TimesBoldItalic,
        BuiltinFontKind::Helvetica => BuiltinFont::Helvetica,
        BuiltinFontKind::HelveticaBold => BuiltinFont::HelveticaBold,
        BuiltinFontKind::HelveticaOblique => BuiltinFont::HelveticaOblique,
        BuiltinFontKind::HelveticaBoldOblique => BuiltinFont::HelveticaBoldOblique,
        BuiltinFontKind::Courier => BuiltinFont::Courier,
        BuiltinFontKind::CourierBold => BuiltinFont::CourierBold,
        BuiltinFontKind::CourierOblique => BuiltinFont::CourierOblique,
        BuiltinFontKind::CourierBoldOblique => BuiltinFont::CourierBoldOblique,
        BuiltinFontKind::Symbol => BuiltinFont::Symbol,
        BuiltinFontKind::ZapfDingbats => BuiltinFont::ZapfDingbats,
    }
}

/// 将 [`FontKey`] 转换为 `printpdf::PdfFontHandle`。
fn to_printpdf_font_handle(
    font: &FontKey,
    custom_fonts: &HashMap<String, FontId>,
) -> PdfFontHandle {
    match font {
        FontKey::Builtin(kind) => PdfFontHandle::Builtin(to_printpdf_builtin(*kind)),
        FontKey::Custom(key) => {
            let font_id = custom_fonts
                .get(key)
                .cloned()
                .unwrap_or_else(|| FontId(key.clone()));
            PdfFontHandle::External(font_id)
        }
    }
}

/// 将 [`LinePointData`] 转换为 `printpdf::LinePoint`。
fn to_printpdf_line_point(pt: &LinePointData) -> LinePoint {
    LinePoint {
        p: Point {
            x: Pt(pt.x as f32),
            y: Pt(pt.y as f32),
        },
        bezier: pt.bezier,
    }
}

/// 将 [`LineData`] 转换为 `printpdf::Line`。
fn to_printpdf_line(line: &LineData) -> Line {
    Line {
        points: line.points.iter().map(to_printpdf_line_point).collect(),
        is_closed: line.is_closed,
    }
}

/// 将 [`XObjectTransformData`] 转换为 `printpdf::XObjectTransform`。
fn to_printpdf_xobject_transform(t: &XObjectTransformData) -> XObjectTransform {
    XObjectTransform {
        translate_x: t.translate_x.map(|v| Pt(v as f32)),
        translate_y: t.translate_y.map(|v| Pt(v as f32)),
        scale_x: t.scale_x,
        scale_y: t.scale_y,
        rotate: None,
        dpi: None,
        no_auto_scale: false,
    }
}

/// 将 [`WriterOp`] 列表转换为 `printpdf::Op` 列表。
///
/// # 参数
///
/// - `ops`：中间表示的操作列表。
/// - `custom_fonts`：自定义字体映射（用户键名 -> printpdf FontId）。
fn to_printpdf_ops(ops: &[WriterOp], custom_fonts: &HashMap<String, FontId>) -> Vec<Op> {
    let mut result = Vec::with_capacity(ops.len());
    for op in ops {
        let printpdf_op = match op {
            WriterOp::StartTextSection => Op::StartTextSection,
            WriterOp::EndTextSection => Op::EndTextSection,
            WriterOp::SetTextCursor { x, y } => Op::SetTextCursor {
                pos: Point {
                    x: Pt(*x as f32),
                    y: Pt(*y as f32),
                },
            },
            WriterOp::SetFont { font, size } => Op::SetFont {
                font: to_printpdf_font_handle(font, custom_fonts),
                size: Pt(*size as f32),
            },
            WriterOp::ShowText { text } => Op::ShowText {
                items: vec![printpdf::TextItem::Text(text.clone())],
            },
            WriterOp::SetOutlineThickness { pt } => Op::SetOutlineThickness { pt: Pt(*pt as f32) },
            WriterOp::DrawLine { line } => Op::DrawLine {
                line: to_printpdf_line(line),
            },
            WriterOp::UseXobject {
                xobject_id,
                transform,
            } => Op::UseXobject {
                id: printpdf::XObjectId(xobject_id.clone()),
                transform: to_printpdf_xobject_transform(transform),
            },
        };
        result.push(printpdf_op);
    }
    result
}

// 需要 ParsedFont 用于字体注册。
use printpdf::ParsedFont;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::resolve_font_key;

    #[test]
    fn resolve_builtin_helvetica() {
        let font = easypdf_core::PdfFont::helvetica(12.0);
        let key = resolve_font_key(&font);
        assert_eq!(key, FontKey::Builtin(BuiltinFontKind::Helvetica));
    }

    #[test]
    fn resolve_builtin_times_bold() {
        let font = easypdf_core::PdfFont::times_roman(12.0).bold();
        let key = resolve_font_key(&font);
        assert_eq!(key, FontKey::Builtin(BuiltinFontKind::TimesBold));
    }

    #[test]
    fn resolve_custom_font() {
        let font = easypdf_core::PdfFont {
            family: easypdf_core::FontFamily::Custom("my_font.ttf".into()),
            size: 12.0,
            style: Default::default(),
        };
        let key = resolve_font_key(&font);
        assert_eq!(key, FontKey::Custom("my_font.ttf".into()));
    }

    #[test]
    fn to_printpdf_builtin_roundtrip() {
        let kinds = [
            BuiltinFontKind::TimesRoman,
            BuiltinFontKind::TimesBold,
            BuiltinFontKind::TimesItalic,
            BuiltinFontKind::TimesBoldItalic,
            BuiltinFontKind::Helvetica,
            BuiltinFontKind::HelveticaBold,
            BuiltinFontKind::HelveticaOblique,
            BuiltinFontKind::HelveticaBoldOblique,
            BuiltinFontKind::Courier,
            BuiltinFontKind::CourierBold,
            BuiltinFontKind::CourierOblique,
            BuiltinFontKind::CourierBoldOblique,
            BuiltinFontKind::Symbol,
            BuiltinFontKind::ZapfDingbats,
        ];
        for kind in kinds {
            let bf = to_printpdf_builtin(kind);
            let _ = format!("{bf:?}");
        }
    }

    #[test]
    fn writerop_serde_roundtrip() {
        let ops = vec![
            WriterOp::StartTextSection,
            WriterOp::SetTextCursor { x: 100.0, y: 700.0 },
            WriterOp::SetFont {
                font: FontKey::Builtin(BuiltinFontKind::Helvetica),
                size: 12.0,
            },
            WriterOp::ShowText {
                text: "Hello".to_string(),
            },
            WriterOp::EndTextSection,
            WriterOp::SetOutlineThickness { pt: 1.0 },
            WriterOp::DrawLine {
                line: LineData {
                    points: vec![
                        LinePointData {
                            x: 0.0,
                            y: 0.0,
                            bezier: false,
                        },
                        LinePointData {
                            x: 100.0,
                            y: 100.0,
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
        ];
        let json = serde_json::to_string(&ops).unwrap();
        let deserialized: Vec<WriterOp> = serde_json::from_str(&json).unwrap();
        assert_eq!(ops, deserialized);
    }
}
