//! 引擎无关的中间操作表示。
//!
//! 定义 [`WriterOp`] 枚举，用于替代直接使用 `printpdf::Op` 作为页面操作的
//! 中间表示。所有 PDF 写入操作先转换为 `WriterOp`，再由具体引擎转换为
//! 后端特定的操作格式。
//!
//! 这种设计将 easypdf-writer 的公共 API 与底层 PDF 库（如 printpdf）解耦。

/// PDF 内置字体的 14 种标准变体。
///
/// 对应 PDF 规范中的标准 Type1 字体集。每种字体独立枚举，
/// 不依赖底层 PDF 库的具体类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) enum BuiltinFontKind {
    /// Times-Roman
    TimesRoman,
    /// Times-Bold
    TimesBold,
    /// Times-Italic
    TimesItalic,
    /// Times-BoldItalic
    TimesBoldItalic,
    /// Helvetica
    Helvetica,
    /// Helvetica-Bold
    HelveticaBold,
    /// Helvetica-Oblique
    HelveticaOblique,
    /// Helvetica-BoldOblique
    HelveticaBoldOblique,
    /// Courier
    Courier,
    /// Courier-Bold
    CourierBold,
    /// Courier-Oblique
    CourierOblique,
    /// Courier-BoldOblique
    CourierBoldOblique,
    /// Symbol
    Symbol,
    /// ZapfDingbats
    ZapfDingbats,
}

/// 字体键，用于在操作中引用字体。
///
/// - `Builtin`：14 种标准 PDF 内置字体之一。
/// - `Custom`：通过 `register_font_from_bytes` 注册的自定义字体的键名。
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) enum FontKey {
    /// 标准 PDF 内置字体。
    Builtin(BuiltinFontKind),
    /// 自定义字体的注册键名。
    Custom(String),
}

/// 线段上的一个点（含贝塞尔控制点标记）。
///
/// 用于描述直线段和贝塞尔曲线段的端点。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LinePointData {
    /// X 坐标（PDF 点）。
    pub x: f64,
    /// Y 坐标（PDF 点）。
    pub y: f64,
    /// 是否为贝塞尔控制点。
    pub bezier: bool,
}

/// 线段数据，包含一系列点和闭合标记。
///
/// 用于表示直线、折线、矩形轮廓和贝塞尔曲线形状。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LineData {
    /// 线段上的点序列。
    pub points: Vec<LinePointData>,
    /// 是否闭合（首尾相连）。
    pub is_closed: bool,
}

/// XObject 变换参数。
///
/// 描述图片或 SVG 在页面上的位置和缩放。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct XObjectTransformData {
    /// X 方向平移（PDF 点）。
    pub translate_x: Option<f64>,
    /// Y 方向平移（PDF 点）。
    pub translate_y: Option<f64>,
    /// X 方向缩放。
    pub scale_x: Option<f32>,
    /// Y 方向缩放。
    pub scale_y: Option<f32>,
}

/// 引擎无关的页面操作中间表示。
///
/// 覆盖当前 easypdf-writer 使用的全部 8 种 `printpdf::Op` 变体。
/// 所有字段类型均为自有类型，不引用 `printpdf` 的类型。
///
/// 通过 [`to_printpdf_ops`](super::printpdf_ops::to_printpdf_ops) 转换为
/// `printpdf::Op` 以生成最终 PDF。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) enum WriterOp {
    /// 开始文本区段（对应 PDF `BT` 操作符）。
    StartTextSection,

    /// 结束文本区段（对应 PDF `ET` 操作符）。
    EndTextSection,

    /// 设置文本光标位置（对应 PDF `Td` 操作符）。
    ///
    /// 坐标原点为页面左下角。
    SetTextCursor {
        /// X 坐标（PDF 点）。
        x: f64,
        /// Y 坐标（PDF 点）。
        y: f64,
    },

    /// 设置当前字体和字号（对应 PDF `Tf` 操作符）。
    SetFont {
        /// 字体引用键。
        font: FontKey,
        /// 字号（PDF 点）。
        size: f64,
    },

    /// 在当前位置显示文本（对应 PDF `Tj` 操作符）。
    ShowText {
        /// 要显示的文本内容。
        text: String,
    },

    /// 设置轮廓线宽度（对应 PDF `w` 操作符）。
    SetOutlineThickness {
        /// 线宽（PDF 点）。
        pt: f64,
    },

    /// 绘制线段或形状（对应 PDF `m`/`l`/`c`/`h` 操作符序列）。
    DrawLine {
        /// 线段数据（含贝塞尔控制点）。
        line: LineData,
    },

    /// 使用已注册的 XObject（图片或 SVG）。
    ///
    /// 调用方需先通过引擎注册图片/SVG 以获取 XObject 引用。
    UseXobject {
        /// XObject 引用标识符（由引擎注册时生成）。
        xobject_id: String,
        /// 变换参数（位置和缩放）。
        transform: XObjectTransformData,
    },
}

/// XObject 资源（图片或 SVG），需要在文档级别注册。
///
/// 在 `write_image` / `write_svg` 时创建，由引擎负责注册到 PDF 文档
/// 并返回 XObject 引用标识符。
pub(crate) enum PendingXObject {
    /// 图片资源（原始字节数据）。
    Image(Vec<u8>),
    /// SVG 资源（XML 字符串数据）。
    Svg(String),
}
