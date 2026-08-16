//! 字体键解析：easypdf-core `PdfFont` → 引擎无关的 [`FontKey`]。

use easypdf_core::{BuiltInFont, FontFamily, PdfFont};

use super::op::{BuiltinFontKind, FontKey};

/// 从 easypdf-core 的 `PdfFont` 解析出 [`FontKey`]。
///
/// 将 `FontFamily` + `FontStyle` 组合解析为具体的内置字体变体，
/// 或标记为自定义字体键。此函数不依赖任何具体引擎类型。
pub(crate) fn resolve_font_key(font: &PdfFont) -> FontKey {
    match &font.family {
        FontFamily::BuiltIn(builtin) => {
            let kind = match builtin {
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
            };
            FontKey::Builtin(kind)
        }
        FontFamily::Custom(path) => FontKey::Custom(path.to_string()),
    }
}
