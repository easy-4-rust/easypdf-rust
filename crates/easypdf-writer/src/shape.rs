//! PdfWriter 的图形绘制方法。

use crate::engine::op::{LineData, LinePointData, WriterOp};
use crate::writer::PdfWriter;

impl PdfWriter {
    /// 在当前页面上绘制线段（坐标从左下角开始）。
    pub fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, line_width: f64) {
        let line = LineData {
            points: vec![
                LinePointData {
                    x: x1,
                    y: y1,
                    bezier: false,
                },
                LinePointData {
                    x: x2,
                    y: y2,
                    bezier: false,
                },
            ],
            is_closed: false,
        };
        self.current_page_ops
            .push(WriterOp::SetOutlineThickness { pt: line_width });
        self.current_page_ops.push(WriterOp::DrawLine { line });
    }

    /// 在当前页面上绘制矩形轮廓。
    pub fn draw_rect_stroke(&mut self, x: f64, y: f64, w: f64, h: f64, line_width: f64) {
        // 构建矩形四角的线段点（与重构前 printpdf::Rect::to_line 行为一致）。
        let line = LineData {
            points: vec![
                LinePointData {
                    x,
                    y,
                    bezier: false,
                },
                LinePointData {
                    x: x + w,
                    y,
                    bezier: false,
                },
                LinePointData {
                    x: x + w,
                    y: y + h,
                    bezier: false,
                },
                LinePointData {
                    x,
                    y: y + h,
                    bezier: false,
                },
            ],
            is_closed: true,
        };
        self.current_page_ops
            .push(WriterOp::SetOutlineThickness { pt: line_width });
        self.current_page_ops.push(WriterOp::DrawLine { line });
    }

    /// 使用 4 条三次贝塞尔曲线绘制圆形轮廓（误差 < 0.027%）。
    pub fn draw_circle(&mut self, cx: f64, cy: f64, radius: f64, line_width: f64) {
        const K: f64 = 0.552_284_749_8;
        let (r, k) = (radius, K * radius);
        #[allow(clippy::type_complexity)]
        let segments: [(f64, f64, f64, f64, f64, f64, f64, f64); 4] = [
            (r, 0.0, r, k, k, r, 0.0, r),
            (0.0, r, -k, r, -r, k, -r, 0.0),
            (-r, 0.0, -r, -k, -k, -r, 0.0, -r),
            (0.0, -r, k, -r, r, -k, r, 0.0),
        ];
        let mut pts = Vec::with_capacity(13);
        for (x1, y1, cx1, cy1, cx2, cy2, x2, y2) in &segments {
            if pts.is_empty() {
                pts.push(LinePointData {
                    x: cx + x1,
                    y: cy + y1,
                    bezier: false,
                });
            }
            pts.push(LinePointData {
                x: cx + cx1,
                y: cy + cy1,
                bezier: true,
            });
            pts.push(LinePointData {
                x: cx + cx2,
                y: cy + cy2,
                bezier: true,
            });
            pts.push(LinePointData {
                x: cx + x2,
                y: cy + y2,
                bezier: false,
            });
        }
        self.current_page_ops
            .push(WriterOp::SetOutlineThickness { pt: line_width });
        self.current_page_ops.push(WriterOp::DrawLine {
            line: LineData {
                points: pts,
                is_closed: true,
            },
        });
    }
}
