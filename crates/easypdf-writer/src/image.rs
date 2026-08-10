//! Image and SVG writing methods for PdfWriter.

use easypdf_core::error::{PdfError, Result};
use easypdf_core::PdfImage;
use crate::writer::PdfWriter;
use printpdf::{Op, Pt, RawImage, XObjectTransform};

impl PdfWriter {
    /// Write an image at (x, y) with dimensions (w, h) in points.
    /// If w=h=0, uses natural pixel size at 72 DPI.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Io` if the image cannot be written.
    pub fn write_image(&mut self, image: &PdfImage, x_pt: f64, y_pt: f64, w_pt: f64, h_pt: f64) -> Result<()> {
        let mut warnings = Vec::new();
        let raw = RawImage::decode_from_bytes(&image.data, &mut warnings)
            .map_err(|e| PdfError::Parse(format!("Image decode error: {e}")))?;
        let xobj_id = self.doc.add_image(&raw);
        let (w, h) = if w_pt == 0.0 && h_pt == 0.0 { (raw.width as f64, raw.height as f64) } else { (w_pt, h_pt) };
        let transform = XObjectTransform {
            translate_x: Some(Pt(x_pt as f32)), translate_y: Some(Pt(y_pt as f32)),
            scale_x: Some(w as f32), scale_y: Some(h as f32),
            rotate: None, dpi: None, no_auto_scale: false,
        };
        self.current_page_ops.push(Op::UseXobject { id: xobj_id, transform });
        Ok(())
    }

    /// Write an SVG at (x, y) with dimensions (w, h) in points.
    pub fn write_svg(&mut self, svg_data: &str, x_pt: f64, y_pt: f64, w_pt: f64, h_pt: f64) -> Result<()> {
        let mut warnings = Vec::new();
        let xobj = printpdf::Svg::parse(svg_data, &mut warnings)
            .map_err(|e| PdfError::Parse(format!("SVG parse error: {e}")))?;
        let xobj_id = self.doc.add_xobject(&xobj);
        let transform = XObjectTransform {
            translate_x: Some(Pt(x_pt as f32)), translate_y: Some(Pt(y_pt as f32)),
            scale_x: Some(w_pt as f32), scale_y: Some(h_pt as f32),
            rotate: None, dpi: None, no_auto_scale: false,
        };
        self.current_page_ops.push(Op::UseXobject { id: xobj_id, transform });
        Ok(())
    }
}
