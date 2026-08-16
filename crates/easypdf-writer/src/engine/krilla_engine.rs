//! 基于 krilla 的写入引擎实现。
//!
//! 提供 [`KrillaEngine`]，实现 [`WriteEngine`](super::write_engine::WriteEngine) trait，
//! 将 [`WriterOp`] 中间表示转换为 krilla 操作并生成最终 PDF。
//!
//! # 内置字体限制
//!
//! krilla 不提供 PDF 标准 14 内置字体的内置支持。使用 `BuiltinFontKind` 时，
//! 引擎将返回明确的错误，要求通过 `register_font` 提供实际字体文件数据。
//!
//! # 坐标系统
//!
//! krilla 使用左上角为原点、Y 轴向下的坐标系统。引擎在内部自动将
//! PDF 坐标系（左下角为原点、Y 轴向上）转换为 krilla 坐标系。

use std::collections::HashMap;

use easypdf_core::PdfMetadata;
use easypdf_core::error::{PdfError, Result};

use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Size, Transform};
use krilla::page::PageSettings;
use krilla::paint::Stroke;
use krilla::text::Font;

use super::op::{
    BuiltinFontKind, FontKey, LineData, PendingXObject, WriterOp, XObjectTransformData,
};
use super::write_engine::WriteEngine;

/// 基于 `krilla` 的 PDF 写入引擎。
///
/// 内部持有 krilla `Document`、字体数据表和待渲染页面，
/// 负责将 [`WriterOp`] 转换为 krilla 操作并生成最终 PDF。
pub(crate) struct KrillaEngine {
    /// 自定义字体数据（键名 -> 原始字节）。
    font_data: HashMap<String, Vec<u8>>,
    /// 已解析的 krilla Font 对象（键名 -> Font）。
    fonts: HashMap<String, Font>,
    /// 图像资源（ID -> 原始字节）。
    images: HashMap<String, Vec<u8>>,
    /// 图像尺寸缓存（ID -> (宽, 高) 像素）。
    image_sizes: HashMap<String, (u32, u32)>,
    /// 图像 ID 计数器。
    image_counter: u64,
    /// 已构建的页面列表（宽度pt, 高度pt, 操作列表）。
    pages: Vec<(f64, f64, Vec<WriterOp>)>,
}

/// 文本状态机，用于跟踪 `apply_ops` 中的文本渲染状态。
struct TextState {
    font: Option<Font>,
    size: f32,
    cursor_x: f32,
    cursor_y: f32,
    buf: String,
    active: bool,
}

impl TextState {
    fn new() -> Self {
        Self {
            font: None,
            size: 12.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            buf: String::new(),
            active: false,
        }
    }

    fn reset(&mut self) {
        self.font = None;
        self.size = 12.0;
        self.cursor_x = 0.0;
        self.cursor_y = 0.0;
        self.buf.clear();
        self.active = false;
    }

    /// 将累积的文本刷新到 surface。
    ///
    /// 将 PDF 坐标系（左下角原点）转换为 krilla 坐标系（左上角原点）。
    fn flush(&mut self, surface: &mut krilla::surface::Surface<'_>, page_height: f32) {
        if self.buf.is_empty() {
            return;
        }
        if let Some(ref font) = self.font {
            // PDF Y -> krilla Y：y_krilla = page_height - y_pdf
            let krilla_y = page_height - self.cursor_y;
            surface.draw_text(
                Point::from_xy(self.cursor_x, krilla_y),
                font.clone(),
                self.size,
                &self.buf,
                false,
                krilla::text::TextDirection::Auto,
            );
        }
        self.buf.clear();
    }
}

impl KrillaEngine {
    /// 创建新的 krilla 引擎。
    pub fn new() -> Self {
        Self {
            font_data: HashMap::new(),
            fonts: HashMap::new(),
            images: HashMap::new(),
            image_sizes: HashMap::new(),
            image_counter: 0,
            pages: Vec::new(),
        }
    }

    /// 解析 [`BuiltinFontKind`] 对应的字体名称。
    ///
    /// krilla 不提供内置 14 字体支持，此方法仅返回字体名称用于错误消息。
    fn builtin_font_name(kind: BuiltinFontKind) -> &'static str {
        match kind {
            BuiltinFontKind::TimesRoman => "Times-Roman",
            BuiltinFontKind::TimesBold => "Times-Bold",
            BuiltinFontKind::TimesItalic => "Times-Italic",
            BuiltinFontKind::TimesBoldItalic => "Times-BoldItalic",
            BuiltinFontKind::Helvetica => "Helvetica",
            BuiltinFontKind::HelveticaBold => "Helvetica-Bold",
            BuiltinFontKind::HelveticaOblique => "Helvetica-Oblique",
            BuiltinFontKind::HelveticaBoldOblique => "Helvetica-BoldOblique",
            BuiltinFontKind::Courier => "Courier",
            BuiltinFontKind::CourierBold => "Courier-Bold",
            BuiltinFontKind::CourierOblique => "Courier-Oblique",
            BuiltinFontKind::CourierBoldOblique => "Courier-BoldOblique",
            BuiltinFontKind::Symbol => "Symbol",
            BuiltinFontKind::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// 静态方法解析字体引用（避免可变借用冲突）。
    fn resolve_font_static(fonts: &HashMap<String, Font>, font_key: &FontKey) -> Result<Font> {
        match font_key {
            FontKey::Builtin(kind) => {
                let name = Self::builtin_font_name(*kind);
                Err(PdfError::UnsupportedFeature(format!(
                    "krilla 引擎不支持 PDF 内置字体 '{name}'。\
                     请通过 register_font 提供实际的 TTF/OTF 字体文件数据。"
                )))
            }
            FontKey::Custom(key) => fonts
                .get(key)
                .cloned()
                .ok_or_else(|| PdfError::Parse(format!("字体 '{key}' 未注册"))),
        }
    }

    /// 将一组 [`WriterOp`] 应用到 krilla surface。
    ///
    /// 处理文本、线条和图像操作，自动将 PDF 坐标系转换为 krilla 坐标系。
    fn apply_ops(
        surface: &mut krilla::surface::Surface<'_>,
        ops: &[WriterOp],
        page_height: f32,
        fonts: &HashMap<String, Font>,
        images: &HashMap<String, Vec<u8>>,
        image_sizes: &HashMap<String, (u32, u32)>,
    ) {
        let mut ts = TextState::new();

        for op in ops {
            match op {
                WriterOp::StartTextSection => {
                    ts.reset();
                    ts.active = true;
                }
                WriterOp::EndTextSection => {
                    ts.flush(surface, page_height);
                    ts.reset();
                }
                WriterOp::SetTextCursor { x, y } => {
                    if ts.active {
                        // 新光标位置：先刷新之前累积的文本。
                        ts.flush(surface, page_height);
                        ts.cursor_x = *x as f32;
                        ts.cursor_y = *y as f32;
                    }
                }
                WriterOp::SetFont {
                    font: font_key,
                    size,
                } => {
                    if ts.active {
                        // 字体变更：先刷新之前累积的文本。
                        ts.flush(surface, page_height);
                        if let Ok(resolved) = Self::resolve_font_static(fonts, font_key) {
                            ts.font = Some(resolved);
                        }
                        ts.size = *size as f32;
                    }
                }
                WriterOp::ShowText { text } => {
                    if ts.active {
                        ts.buf.push_str(text);
                    }
                }
                WriterOp::SetOutlineThickness { pt } => {
                    let stroke = Stroke {
                        paint: rgb::Color::new(0, 0, 0).into(),
                        width: *pt as f32,
                        ..Stroke::default()
                    };
                    surface.set_stroke(Some(stroke));
                }
                WriterOp::DrawLine { line } => {
                    if let Some(path) = Self::build_path(line, page_height) {
                        surface.draw_path(&path);
                    }
                }
                WriterOp::UseXobject {
                    xobject_id,
                    transform,
                } => {
                    Self::apply_xobject(
                        surface,
                        xobject_id,
                        transform,
                        page_height,
                        images,
                        image_sizes,
                    );
                }
            }
        }

        // 安全清理：如果仍在文本区段中，刷新剩余文本。
        if ts.active {
            ts.flush(surface, page_height);
        }
    }

    /// 将 [`LineData`] 转换为 krilla `Path`。
    ///
    /// 坐标从 PDF 坐标系（左下角原点）转换为 krilla 坐标系（左上角原点）。
    fn build_path(line: &LineData, page_height: f32) -> Option<krilla::geom::Path> {
        if line.points.is_empty() {
            return None;
        }

        let mut builder = PathBuilder::new();
        let mut i = 0;

        while i < line.points.len() {
            let pt = &line.points[i];
            let x = pt.x as f32;
            let y = page_height - pt.y as f32;

            if i == 0 {
                builder.move_to(x, y);
                i += 1;
            } else if pt.bezier && i + 2 < line.points.len() {
                // 三次贝塞尔曲线：两个控制点 + 一个终点。
                let cp1 = &line.points[i];
                let cp2 = &line.points[i + 1];
                let end = &line.points[i + 2];
                builder.cubic_to(
                    cp1.x as f32,
                    page_height - cp1.y as f32,
                    cp2.x as f32,
                    page_height - cp2.y as f32,
                    end.x as f32,
                    page_height - end.y as f32,
                );
                i += 3;
            } else {
                builder.line_to(x, y);
                i += 1;
            }
        }

        if line.is_closed {
            builder.close();
        }

        builder.finish()
    }

    /// 应用 XObject（图像）到 surface。
    fn apply_xobject(
        surface: &mut krilla::surface::Surface<'_>,
        xobject_id: &str,
        transform: &XObjectTransformData,
        page_height: f32,
        images: &HashMap<String, Vec<u8>>,
        image_sizes: &HashMap<String, (u32, u32)>,
    ) {
        let Some(data) = images.get(xobject_id) else {
            return;
        };

        let (native_w, native_h) = image_sizes.get(xobject_id).copied().unwrap_or((1, 1));

        // 目标尺寸（点）。
        let w_pt = transform.scale_x.unwrap_or(native_w as f32);
        let h_pt = transform.scale_y.unwrap_or(native_h as f32);

        // 位置（PDF 坐标系 -> krilla 坐标系）。
        // PDF: (x, y) 是图片左下角。krilla: (x, y) 是图片左上角。
        let x_pt = transform.translate_x.map_or(0.0, |v| v as f32);
        let y_pt = transform.translate_y.map_or(0.0, |v| v as f32);
        let krilla_y = page_height - y_pt - h_pt;

        // 从数据头部检测图像格式。
        let image =
            if data.len() >= 8 && data[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
                krilla::image::Image::from_png(data.clone().into(), false)
            } else if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
                krilla::image::Image::from_jpeg(data.clone().into(), false)
            } else {
                // 不支持的图像格式，跳过。
                return;
            };

        if let Ok(img) = image {
            let translate = Transform::from_translate(x_pt, krilla_y);
            surface.push_transform(&translate);
            surface.draw_image(
                img,
                Size::from_wh(w_pt, h_pt).unwrap_or_else(|| Size::from_wh(1.0, 1.0).unwrap()),
            );
            surface.pop();
        }
    }

    /// 将 [`PdfMetadata`] 转换为 krilla `Metadata`。
    fn to_krilla_metadata(metadata: &PdfMetadata) -> krilla::metadata::Metadata {
        let mut km = krilla::metadata::Metadata::new();
        if let Some(ref title) = metadata.title {
            km = km.title(title.clone());
        }
        if let Some(ref author) = metadata.author {
            km = km.authors(vec![author.clone()]);
        }
        if let Some(ref creator) = metadata.creator {
            km = km.creator(creator.clone());
        }
        if let Some(ref producer) = metadata.producer {
            km = km.producer(producer.clone());
        }
        if let Some(ref keywords) = metadata.keywords {
            let kw_vec: Vec<String> = keywords.split(',').map(|s| s.trim().to_string()).collect();
            km = km.keywords(kw_vec);
        }
        km
    }

    /// 解析图像数据获取原始尺寸（像素）。
    fn decode_image_size(data: &[u8]) -> Result<(u32, u32)> {
        // PNG 魔数。
        if data.len() >= 24 && data[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
            let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
            return Ok((w, h));
        }
        // JPEG 魔数。
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
            let mut warnings = Vec::new();
            let raw = printpdf::RawImage::decode_from_bytes(data, &mut warnings)
                .map_err(|e| PdfError::Parse(format!("JPEG 尺寸解析错误: {e}")))?;
            return Ok((raw.width as u32, raw.height as u32));
        }
        Err(PdfError::Parse(
            "不支持的图像格式（仅支持 PNG 和 JPEG）".to_string(),
        ))
    }
}

impl WriteEngine for KrillaEngine {
    fn register_font(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let font = Font::new(data.to_vec().into(), 0)
            .ok_or_else(|| PdfError::Parse(format!("krilla 无法解析字体数据: {key}")))?;
        self.font_data.insert(key.to_string(), data.to_vec());
        self.fonts.insert(key.to_string(), font);
        Ok(())
    }

    fn register_xobject(&mut self, xobject: PendingXObject) -> Result<String> {
        self.image_counter += 1;
        let id = format!("img_{}", self.image_counter);

        match xobject {
            PendingXObject::Image(data) => {
                let (w, h) = Self::decode_image_size(&data)?;
                self.image_sizes.insert(id.clone(), (w, h));
                self.images.insert(id.clone(), data);
            }
            PendingXObject::Svg(_) => {
                return Err(PdfError::UnsupportedFeature(
                    "krilla 引擎不支持 SVG XObject。请使用 PNG 或 JPEG 图像。".to_string(),
                ));
            }
        }

        Ok(id)
    }

    fn add_page(&mut self, width_pt: f64, height_pt: f64, ops: Vec<WriterOp>) {
        self.pages.push((width_pt, height_pt, ops));
    }

    fn finish(&mut self, metadata: &PdfMetadata) -> Result<Vec<u8>> {
        let mut doc = krilla::Document::new();

        // 应用文档元数据。
        let krilla_meta = Self::to_krilla_metadata(metadata);
        doc.set_metadata(krilla_meta);

        // 渲染所有页面。
        let pages = std::mem::take(&mut self.pages);
        for (width_pt, height_pt, ops) in pages {
            let settings =
                PageSettings::from_wh(width_pt as f32, height_pt as f32).ok_or_else(|| {
                    PdfError::Parse(format!("无效的页面尺寸: {width_pt}x{height_pt}"))
                })?;

            let mut page = doc.start_page_with(settings);
            {
                let mut surface = page.surface();
                Self::apply_ops(
                    &mut surface,
                    &ops,
                    height_pt as f32,
                    &self.fonts,
                    &self.images,
                    &self.image_sizes,
                );
                surface.finish();
            }
            page.finish();
        }

        doc.finish()
            .map_err(|e| PdfError::Parse(format!("krilla 文档序列化失败: {e}")))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn builtin_font_name_returns_correct_name() {
        assert_eq!(
            KrillaEngine::builtin_font_name(BuiltinFontKind::Helvetica),
            "Helvetica"
        );
        assert_eq!(
            KrillaEngine::builtin_font_name(BuiltinFontKind::TimesBold),
            "Times-Bold"
        );
        assert_eq!(
            KrillaEngine::builtin_font_name(BuiltinFontKind::ZapfDingbats),
            "ZapfDingbats"
        );
    }

    #[test]
    fn resolve_builtin_returns_error() {
        let fonts = HashMap::new();
        let result = KrillaEngine::resolve_font_static(
            &fonts,
            &FontKey::Builtin(BuiltinFontKind::Helvetica),
        );
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("不支持"));
        assert!(err_msg.contains("Helvetica"));
    }

    #[test]
    fn resolve_unregistered_custom_returns_error() {
        let fonts = HashMap::new();
        let result = KrillaEngine::resolve_font_static(&fonts, &FontKey::Custom("missing".into()));
        assert!(result.is_err());
    }

    #[test]
    fn build_path_empty_points_returns_none() {
        let line = LineData {
            points: vec![],
            is_closed: false,
        };
        assert!(KrillaEngine::build_path(&line, 842.0).is_none());
    }

    #[test]
    fn build_path_simple_line() {
        use super::super::op::LinePointData;
        let line = LineData {
            points: vec![
                LinePointData {
                    x: 10.0,
                    y: 10.0,
                    bezier: false,
                },
                LinePointData {
                    x: 100.0,
                    y: 10.0,
                    bezier: false,
                },
            ],
            is_closed: false,
        };
        let path = KrillaEngine::build_path(&line, 842.0);
        assert!(path.is_some());
    }

    #[test]
    fn build_path_closed_rect() {
        use super::super::op::LinePointData;
        let line = LineData {
            points: vec![
                LinePointData {
                    x: 0.0,
                    y: 0.0,
                    bezier: false,
                },
                LinePointData {
                    x: 100.0,
                    y: 0.0,
                    bezier: false,
                },
                LinePointData {
                    x: 100.0,
                    y: 50.0,
                    bezier: false,
                },
                LinePointData {
                    x: 0.0,
                    y: 50.0,
                    bezier: false,
                },
            ],
            is_closed: true,
        };
        let path = KrillaEngine::build_path(&line, 842.0);
        assert!(path.is_some());
    }

    #[test]
    fn decode_image_size_png() {
        let data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
            0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let (w, h) = KrillaEngine::decode_image_size(&data).unwrap();
        assert_eq!(w, 1);
        assert_eq!(h, 1);
    }

    #[test]
    fn decode_image_size_unsupported_format() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        assert!(KrillaEngine::decode_image_size(&data).is_err());
    }

    #[test]
    fn to_krilla_metadata_title_and_author() {
        let meta = PdfMetadata::new().title("Test").author("Author");
        let km = KrillaEngine::to_krilla_metadata(&meta);
        let _ = format!("{km:?}");
    }

    #[test]
    fn krilla_engine_new_creates_empty_state() {
        let engine = KrillaEngine::new();
        assert!(engine.font_data.is_empty());
        assert!(engine.fonts.is_empty());
        assert!(engine.images.is_empty());
        assert!(engine.pages.is_empty());
    }

    #[test]
    fn add_page_stores_ops() {
        let mut engine = KrillaEngine::new();
        let ops = vec![WriterOp::StartTextSection, WriterOp::EndTextSection];
        engine.add_page(595.0, 842.0, ops);
        assert_eq!(engine.pages.len(), 1);
        assert_eq!(engine.pages[0].0, 595.0);
        assert_eq!(engine.pages[0].1, 842.0);
    }

    #[test]
    fn register_xobject_svg_returns_error() {
        let mut engine = KrillaEngine::new();
        let result = engine.register_xobject(PendingXObject::Svg("<svg/>".to_string()));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("SVG"));
    }

    #[test]
    fn finish_empty_document_produces_pdf() {
        let mut engine = KrillaEngine::new();
        engine.add_page(595.0, 842.0, vec![]);
        let meta = PdfMetadata::default();
        let result = engine.finish(&meta);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }
}
