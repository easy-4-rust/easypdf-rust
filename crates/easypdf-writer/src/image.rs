//! PdfWriter 的图片和 SVG 写入方法。

use crate::engine::PendingXObject;
use crate::engine::op::{WriterOp, XObjectTransformData};
use crate::writer::PdfWriter;
use easypdf_core::PdfImage;
use easypdf_core::error::{PdfError, Result};
use printpdf::RawImage;

impl PdfWriter {
    /// 在点坐标 (x, y) 处写入尺寸为 (w, h) 的图片。
    /// 如果 w=h=0，则使用 72 DPI 下的原始像素尺寸。
    ///
    /// # Errors
    ///
    /// 当图片无法写入时返回 `PdfError::Io`。
    pub fn write_image(
        &mut self,
        image: &PdfImage,
        x_pt: f64,
        y_pt: f64,
        w_pt: f64,
        h_pt: f64,
    ) -> Result<()> {
        // 预验证图片数据（保持与重构前一致的错误语义）。
        let mut warnings = Vec::new();
        let raw = RawImage::decode_from_bytes(&image.data, &mut warnings)
            .map_err(|e| PdfError::Parse(format!("Image decode error: {e}")))?;

        let (w, h) = if w_pt == 0.0 && h_pt == 0.0 {
            (raw.width as f64, raw.height as f64)
        } else {
            (w_pt, h_pt)
        };

        let xobject_id = self
            .engine
            .register_xobject(PendingXObject::Image(image.data.clone()))?;

        let transform = XObjectTransformData {
            translate_x: Some(x_pt),
            translate_y: Some(y_pt),
            scale_x: Some(w as f32),
            scale_y: Some(h as f32),
        };
        self.current_page_ops.push(WriterOp::UseXobject {
            xobject_id,
            transform,
        });
        Ok(())
    }

    /// 在点坐标 (x, y) 处写入尺寸为 (w, h) 的 SVG。
    pub fn write_svg(
        &mut self,
        svg_data: &str,
        x_pt: f64,
        y_pt: f64,
        w_pt: f64,
        h_pt: f64,
    ) -> Result<()> {
        let xobject_id = self
            .engine
            .register_xobject(PendingXObject::Svg(svg_data.to_string()))?;

        let transform = XObjectTransformData {
            translate_x: Some(x_pt),
            translate_y: Some(y_pt),
            scale_x: Some(w_pt as f32),
            scale_y: Some(h_pt as f32),
        };
        self.current_page_ops.push(WriterOp::UseXobject {
            xobject_id,
            transform,
        });
        Ok(())
    }
}
