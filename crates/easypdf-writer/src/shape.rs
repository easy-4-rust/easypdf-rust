//! PdfWriter 的图形绘制方法。

use crate::writer::PdfWriter;
use printpdf::{Line, LinePoint, Op, Point, Pt};

impl PdfWriter {
    /// 在当前页面上绘制线段（坐标从左下角开始）。
    pub fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, line_width: f64) {
        let line = Line {
            points: vec![
                LinePoint {
                    p: Point {
                        x: Pt(x1 as f32),
                        y: Pt(y1 as f32),
                    },
                    bezier: false,
                },
                LinePoint {
                    p: Point {
                        x: Pt(x2 as f32),
                        y: Pt(y2 as f32),
                    },
                    bezier: false,
                },
            ],
            is_closed: false,
        };
        self.current_page_ops.push(Op::SetOutlineThickness {
            pt: Pt(line_width as f32),
        });
        self.current_page_ops.push(Op::DrawLine { line });
    }

    /// 在当前页面上绘制矩形轮廓。
    pub fn draw_rect_stroke(&mut self, x: f64, y: f64, w: f64, h: f64, line_width: f64) {
        let rect = printpdf::Rect::from_wh(Pt(w as f32), Pt(h as f32));
        let line = rect.to_line();
        let translated = Line {
            points: line
                .points
                .into_iter()
                .map(|mut lp| {
                    lp.p.x = Pt(lp.p.x.0 + x as f32);
                    lp.p.y = Pt(lp.p.y.0 + y as f32);
                    lp
                })
                .collect(),
            is_closed: line.is_closed,
        };
        self.current_page_ops.push(Op::SetOutlineThickness {
            pt: Pt(line_width as f32),
        });
        self.current_page_ops
            .push(Op::DrawLine { line: translated });
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
                pts.push(LinePoint {
                    p: Point {
                        x: Pt((cx + x1) as f32),
                        y: Pt((cy + y1) as f32),
                    },
                    bezier: false,
                });
            }
            pts.push(LinePoint {
                p: Point {
                    x: Pt((cx + cx1) as f32),
                    y: Pt((cy + cy1) as f32),
                },
                bezier: true,
            });
            pts.push(LinePoint {
                p: Point {
                    x: Pt((cx + cx2) as f32),
                    y: Pt((cy + cy2) as f32),
                },
                bezier: true,
            });
            pts.push(LinePoint {
                p: Point {
                    x: Pt((cx + x2) as f32),
                    y: Pt((cy + y2) as f32),
                },
                bezier: false,
            });
        }
        self.current_page_ops.push(Op::SetOutlineThickness {
            pt: Pt(line_width as f32),
        });
        self.current_page_ops.push(Op::DrawLine {
            line: Line {
                points: pts,
                is_closed: true,
            },
        });
    }
}
