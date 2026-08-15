//! Enumerations used throughout `easypdf-rust`.

/// Standard page sizes in PDF points (1 point = 1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    /// A0 (2384 × 3370 pt)
    A0,
    /// A1 (1684 × 2384 pt)
    A1,
    /// A2 (1191 × 1684 pt)
    A2,
    /// A3 (842 × 1191 pt)
    A3,
    /// A4 (595 × 842 pt)
    A4,
    /// A5 (420 × 595 pt)
    A5,
    /// US Letter (612 × 792 pt)
    Letter,
    /// US Legal (612 × 1008 pt)
    Legal,
    /// Custom page size in points (width, height).
    Custom(f64, f64),
}

impl PageSize {
    /// Returns the dimensions of this page size as `(width, height)` in PDF points.
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

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Portrait (height > width).
    #[default]
    Portrait,
    /// Landscape (width > height).
    Landscape,
}

/// Rotation angle in degrees (clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// No rotation.
    None,
    /// Rotate 90° clockwise.
    Clockwise90,
    /// Rotate 180°.
    Clockwise180,
    /// Rotate 270° clockwise (equivalent to 90° counter-clockwise).
    Clockwise270,
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    /// Align text to the left.
    #[default]
    Left,
    /// Center text horizontally.
    Center,
    /// Align text to the right.
    Right,
    /// Justify text (stretch to fill the line width).
    Justify,
}

/// Vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// Align to the top.
    #[default]
    Top,
    /// Center vertically.
    Middle,
    /// Align to the bottom.
    Bottom,
}

/// Supported image formats for embedding in PDFs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG image.
    Jpeg,
    /// PNG image.
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
