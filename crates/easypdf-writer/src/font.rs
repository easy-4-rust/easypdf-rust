//! 字体映射与字体注册逻辑。

use easypdf_core::PdfFont;
use printpdf::BuiltinFont;

use crate::engine::op::BuiltinFontKind;

/// 将 easypdf-core 的 FontFamily 映射到 printpdf 的 BuiltinFont。
#[must_use]
pub fn map_builtin_font(font: &PdfFont) -> BuiltinFont {
    let kind = resolve_builtin_kind(font);
    builtin_kind_to_printpdf(kind)
}

/// 从 `PdfFont` 解析出内置字体种类。
///
/// 仅处理内置字体；自定义字体回退到 Helvetica 系列。
pub(crate) fn resolve_builtin_kind(font: &PdfFont) -> BuiltinFontKind {
    use easypdf_core::{BuiltInFont, FontFamily};

    match &font.family {
        FontFamily::BuiltIn(builtin) => match builtin {
            BuiltInFont::Helvetica
            | BuiltInFont::HelveticaBold
            | BuiltInFont::HelveticaOblique
            | BuiltInFont::HelveticaBoldOblique => {
                if font.style.bold && font.style.italic {
                    BuiltinFontKind::HelveticaBoldOblique
                } else if font.style.bold {
                    BuiltinFontKind::HelveticaBold
                } else if font.style.italic {
                    BuiltinFontKind::HelveticaOblique
                } else {
                    BuiltinFontKind::Helvetica
                }
            }
            BuiltInFont::TimesRoman
            | BuiltInFont::TimesBold
            | BuiltInFont::TimesItalic
            | BuiltInFont::TimesBoldItalic => {
                if font.style.bold && font.style.italic {
                    BuiltinFontKind::TimesBoldItalic
                } else if font.style.bold {
                    BuiltinFontKind::TimesBold
                } else if font.style.italic {
                    BuiltinFontKind::TimesItalic
                } else {
                    BuiltinFontKind::TimesRoman
                }
            }
            BuiltInFont::Courier
            | BuiltInFont::CourierBold
            | BuiltInFont::CourierOblique
            | BuiltInFont::CourierBoldOblique => {
                if font.style.bold && font.style.italic {
                    BuiltinFontKind::CourierBoldOblique
                } else if font.style.bold {
                    BuiltinFontKind::CourierBold
                } else if font.style.italic {
                    BuiltinFontKind::CourierOblique
                } else {
                    BuiltinFontKind::Courier
                }
            }
            BuiltInFont::Symbol => BuiltinFontKind::Symbol,
            BuiltInFont::ZapfDingbats => BuiltinFontKind::ZapfDingbats,
        },
        FontFamily::Custom(_) => {
            // 自定义字体无法映射到内置字体，回退到 Helvetica。
            if font.style.bold {
                BuiltinFontKind::HelveticaBold
            } else {
                BuiltinFontKind::Helvetica
            }
        }
    }
}

/// 将 `BuiltinFontKind` 转换为 `printpdf::BuiltinFont`。
fn builtin_kind_to_printpdf(kind: BuiltinFontKind) -> BuiltinFont {
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
