//! Font mapping and font registration logic.

use easypdf_core::{BuiltInFont, FontFamily, PdfFont};
use printpdf::BuiltinFont;

/// Map easypdf-core's FontFamily to printpdf's BuiltinFont.
pub fn map_builtin_font(font: &PdfFont) -> BuiltinFont {
    match &font.family {
        FontFamily::BuiltIn(builtin) => match builtin {
            BuiltInFont::Helvetica | BuiltInFont::HelveticaBold | BuiltInFont::HelveticaOblique | BuiltInFont::HelveticaBoldOblique => {
                if font.style.bold && font.style.italic { BuiltinFont::HelveticaBoldOblique }
                else if font.style.bold { BuiltinFont::HelveticaBold }
                else if font.style.italic { BuiltinFont::HelveticaOblique }
                else { BuiltinFont::Helvetica }
            }
            BuiltInFont::TimesRoman | BuiltInFont::TimesBold | BuiltInFont::TimesItalic | BuiltInFont::TimesBoldItalic => {
                if font.style.bold && font.style.italic { BuiltinFont::TimesBoldItalic }
                else if font.style.bold { BuiltinFont::TimesBold }
                else if font.style.italic { BuiltinFont::TimesItalic }
                else { BuiltinFont::TimesRoman }
            }
            BuiltInFont::Courier | BuiltInFont::CourierBold | BuiltInFont::CourierOblique | BuiltInFont::CourierBoldOblique => {
                if font.style.bold && font.style.italic { BuiltinFont::CourierBoldOblique }
                else if font.style.bold { BuiltinFont::CourierBold }
                else if font.style.italic { BuiltinFont::CourierOblique }
                else { BuiltinFont::Courier }
            }
            BuiltInFont::Symbol => BuiltinFont::Symbol,
            BuiltInFont::ZapfDingbats => BuiltinFont::ZapfDingbats,
        },
        FontFamily::Custom(_) => {
            if font.style.bold { BuiltinFont::HelveticaBold } else { BuiltinFont::Helvetica }
        }
    }
}
