//! `easypdf-rust` 中使用的枚举类型。

/// 标准页面尺寸（PDF 点，1 点 = 1/72 英寸）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    /// A0（2384 x 3370 点）
    A0,
    /// A1（1684 x 2384 点）
    A1,
    /// A2（1191 x 1684 点）
    A2,
    /// A3（842 x 1191 点）
    A3,
    /// A4（595 x 842 点）
    A4,
    /// A5（420 x 595 点）
    A5,
    /// US Letter（612 x 792 点）
    Letter,
    /// US Legal（612 x 1008 点）
    Legal,
    /// 自定义页面尺寸（宽度、高度，单位为点）。
    Custom(f64, f64),
}

impl PageSize {
    /// 返回此页面尺寸的 `(宽度, 高度)`，单位为 PDF 点。
    #[must_use]
    pub const fn dimensions(self) -> (f64, f64) {
        match self {
            Self::A0 => (2384.0, 3370.0),
            Self::A1 => (1684.0, 2384.0),
            Self::A2 => (1191.0, 1684.0),
            Self::A3 => (842.0, 1191.0),
            Self::A4 => (595.0, 842.0),
            Self::A5 => (420.0, 595.0),
            Self::Letter => (612.0, 792.0),
            Self::Legal => (612.0, 1008.0),
            Self::Custom(w, h) => (w, h),
        }
    }
}

/// 页面方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// 纵向（高度 > 宽度）。
    #[default]
    Portrait,
    /// 横向（宽度 > 高度）。
    Landscape,
}

/// 旋转角度（顺时针，单位为度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// 不旋转。
    None,
    /// 顺时针旋转 90 度。
    Clockwise90,
    /// 旋转 180 度。
    Clockwise180,
    /// 顺时针旋转 270 度（等价于逆时针 90 度）。
    Clockwise270,
}

/// 水平文本对齐方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    /// 文本左对齐。
    #[default]
    Left,
    /// 文本水平居中。
    Center,
    /// 文本右对齐。
    Right,
    /// 文本两端对齐（拉伸以填满行宽）。
    Justify,
}

/// 垂直对齐方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// 顶部对齐。
    #[default]
    Top,
    /// 垂直居中。
    Middle,
    /// 底部对齐。
    Bottom,
}

/// 支持嵌入 PDF 的图片格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG 图片。
    Jpeg,
    /// PNG 图片。
    Png,
}

#[cfg(test)]
#[allow(clippy::uninlined_format_args, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn page_size_a4() {
        let (w, h) = PageSize::A4.dimensions();
        assert_eq!(w, 595.0);
        assert_eq!(h, 842.0);
    }

    #[test]
    fn page_size_a0() {
        let (w, _h) = PageSize::A0.dimensions();
        assert_eq!(w, 2384.0);
    }

    #[test]
    fn page_size_letter() {
        let (w, h) = PageSize::Letter.dimensions();
        assert_eq!(w, 612.0);
        assert_eq!(h, 792.0);
    }

    #[test]
    fn page_size_legal() {
        let (_, h) = PageSize::Legal.dimensions();
        assert_eq!(h, 1008.0);
    }

    #[test]
    fn page_size_custom() {
        let (w, h) = PageSize::Custom(300.0, 400.0).dimensions();
        assert_eq!(w, 300.0);
        assert_eq!(h, 400.0);
    }

    #[test]
    fn orientation_default_is_portrait() {
        assert_eq!(Orientation::default(), Orientation::Portrait);
    }

    #[test]
    fn orientation_debug() {
        assert_eq!(format!("{:?}", Orientation::Landscape), "Landscape");
    }

    #[test]
    fn text_alignment_default_is_left() {
        assert_eq!(TextAlignment::default(), TextAlignment::Left);
    }

    #[test]
    fn text_alignment_variants() {
        assert_ne!(TextAlignment::Left, TextAlignment::Center);
        assert_ne!(TextAlignment::Right, TextAlignment::Justify);
    }

    #[test]
    fn vertical_alignment_default_is_top() {
        assert_eq!(VerticalAlignment::default(), VerticalAlignment::Top);
    }

    #[test]
    fn rotation_variants() {
        assert_ne!(Rotation::None, Rotation::Clockwise90);
        assert_ne!(Rotation::Clockwise180, Rotation::Clockwise270);
    }

    #[test]
    fn image_format_variants() {
        assert_ne!(ImageFormat::Jpeg, ImageFormat::Png);
    }

    #[test]
    fn page_size_clone_copy() {
        let ps = PageSize::A4;
        let copied = ps;
        assert_eq!(ps, copied);
    }

    #[test]
    fn page_size_custom_eq() {
        assert_eq!(PageSize::Custom(1.0, 2.0), PageSize::Custom(1.0, 2.0));
        assert_ne!(PageSize::Custom(1.0, 2.0), PageSize::Custom(3.0, 4.0));
    }

    #[test]
    fn page_size_all_sizes() {
        let sizes = [
            PageSize::A0,
            PageSize::A1,
            PageSize::A2,
            PageSize::A3,
            PageSize::A4,
            PageSize::A5,
            PageSize::Letter,
            PageSize::Legal,
        ];
        for s in sizes {
            let (w, h) = s.dimensions();
            assert!(w > 0.0);
            assert!(h > 0.0);
        }
    }
}
