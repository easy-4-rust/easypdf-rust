//! [`PdfConverter`] 实例的类型擦除转换器注册表。
//!
//! 提供 [`ConverterRegistry`]，使用 [`TypeId`] 进行运行时分发，
//! 将 Rust 类型映射到其 [`PdfConverter`] 实现。
//!
//! # Examples
//!
//! ```
//! use easypdf_core::converter_registry::ConverterRegistry;
//!
//! let registry = ConverterRegistry::with_defaults();
//! let s = registry.to_pdf_string(&42_i64).unwrap();
//! assert_eq!(s, "42");
//!
//! let n: i64 = registry.from_pdf_string("99").unwrap();
//! assert_eq!(n, 99);
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::error::{PdfError, Result};
use crate::traits::PdfConverter;

// ---------------------------------------------------------------------------
// 类型擦除转换器 trait
// ---------------------------------------------------------------------------

/// 擦除 [`PdfConverter<T>`] 泛型参数的内部 trait。
trait ErasedConverter: Send {
    /// 将值转换为其 PDF 字符串表示。
    ///
    /// # Errors
    ///
    /// 值无法表示时返回错误。
    fn erased_to_pdf_string(&self, value: &dyn Any) -> Result<String>;

    /// 将 PDF 字符串表示转换回 Rust 值。
    ///
    /// # Errors
    ///
    /// 字符串无法解析时返回错误。
    fn erased_from_pdf_string(&self, s: &str) -> Result<Box<dyn Any>>;
}

/// 包装 `PdfConverter<T>` 以擦除其类型参数的适配器。
struct ConverterAdapter<T> {
    inner: Box<dyn PdfConverter<T>>,
}

impl<T: 'static> ErasedConverter for ConverterAdapter<T> {
    fn erased_to_pdf_string(&self, value: &dyn Any) -> Result<String> {
        let typed = value.downcast_ref::<T>().ok_or_else(|| {
            PdfError::Other(format!(
                "type mismatch: converter expects {}, got a different type",
                std::any::type_name::<T>()
            ))
        })?;
        PdfConverter::to_pdf_string(self.inner.as_ref(), typed)
    }

    fn erased_from_pdf_string(&self, s: &str) -> Result<Box<dyn Any>> {
        let value = PdfConverter::from_pdf_string(self.inner.as_ref(), s)?;
        Ok(Box::new(value))
    }
}

// ---------------------------------------------------------------------------
// ConverterRegistry
// ---------------------------------------------------------------------------

/// 将 Rust 类型映射到其 [`PdfConverter`] 实现的运行时注册表。
///
/// 转换器以类型擦除的形式存储，以 [`TypeId`] 为键。每种类型 `T`
/// 最多注册一个转换器；后续注册会替换先前的。
///
/// 使用 [`ConverterRegistry::with_defaults`] 获取预填充了常见类型
///（`String`、`i64`、`f64`、`bool`、`chrono::DateTime<chrono::Utc>`）
/// 转换器的注册表。
pub struct ConverterRegistry {
    converters: HashMap<TypeId, Box<dyn ErasedConverter>>,
}

impl ConverterRegistry {
    /// 创建没有转换器的空注册表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            converters: HashMap::new(),
        }
    }

    /// 创建预填充了常见类型转换器的注册表。
    ///
    /// 包含以下类型的转换器：`String`、`i64`、`f64`、`bool`、
    /// `chrono::DateTime<chrono::Utc>`。
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register::<String>(Box::new(StringConverter));
        registry.register::<i64>(Box::new(I64Converter));
        registry.register::<f64>(Box::new(F64Converter));
        registry.register::<bool>(Box::new(BoolConverter));
        registry.register::<chrono::DateTime<chrono::Utc>>(Box::new(ChronoUtcConverter));
        registry
    }

    /// 为类型 `T` 注册转换器。
    ///
    /// 如果已为 `T` 注册了转换器，则替换之。
    pub fn register<T: 'static>(&mut self, converter: Box<dyn PdfConverter<T>>) {
        let erased: Box<dyn ErasedConverter> = Box::new(ConverterAdapter { inner: converter });
        self.converters.insert(TypeId::of::<T>(), erased);
    }

    /// 返回是否为类型 `T` 注册了转换器。
    #[must_use]
    pub fn has<T: 'static>(&self) -> bool {
        self.converters.contains_key(&TypeId::of::<T>())
    }

    /// 返回已注册转换器的数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.converters.len()
    }

    /// 返回注册表是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.converters.is_empty()
    }

    /// 将类型 `T` 的值转换为其 PDF 字符串表示。
    ///
    /// # Errors
    ///
    /// 未为 `T` 注册转换器时返回 [`PdfError::UnsupportedFeature`]，
    /// 或传播转换器的错误。
    pub fn to_pdf_string<T: 'static>(&self, value: &T) -> Result<String> {
        let converter = self.converters.get(&TypeId::of::<T>()).ok_or_else(|| {
            PdfError::UnsupportedFeature(format!(
                "no converter registered for {}",
                std::any::type_name::<T>()
            ))
        })?;
        converter.erased_to_pdf_string(value as &dyn Any)
    }

    /// 将 PDF 字符串表示转换回类型 `T` 的值。
    ///
    /// # Errors
    ///
    /// 未为 `T` 注册转换器时返回 [`PdfError::UnsupportedFeature`]，
    /// 或传播转换器的错误。
    pub fn from_pdf_string<T: 'static>(&self, s: &str) -> Result<T> {
        let converter = self.converters.get(&TypeId::of::<T>()).ok_or_else(|| {
            PdfError::UnsupportedFeature(format!(
                "no converter registered for {}",
                std::any::type_name::<T>()
            ))
        })?;
        let boxed = converter.erased_from_pdf_string(s)?;
        boxed.downcast::<T>().map(|v| *v).map_err(|_| {
            PdfError::Other(format!(
                "converter returned wrong type for {}",
                std::any::type_name::<T>()
            ))
        })
    }
}

impl Default for ConverterRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ---------------------------------------------------------------------------
// 内置转换器
// ---------------------------------------------------------------------------

/// `String` 的转换器——恒等变换。
struct StringConverter;

impl PdfConverter<String> for StringConverter {
    fn to_pdf_string(&self, value: &String) -> Result<String> {
        Ok(value.clone())
    }

    fn from_pdf_string(&self, s: &str) -> Result<String> {
        Ok(s.to_owned())
    }
}

/// `i64` 的转换器。
struct I64Converter;

impl PdfConverter<i64> for I64Converter {
    fn to_pdf_string(&self, value: &i64) -> Result<String> {
        Ok(value.to_string())
    }

    fn from_pdf_string(&self, s: &str) -> Result<i64> {
        s.parse::<i64>()
            .map_err(|e| PdfError::Parse(format!("invalid i64 '{s}': {e}")))
    }
}

/// `f64` 的转换器。
struct F64Converter;

impl PdfConverter<f64> for F64Converter {
    fn to_pdf_string(&self, value: &f64) -> Result<String> {
        Ok(value.to_string())
    }

    fn from_pdf_string(&self, s: &str) -> Result<f64> {
        s.parse::<f64>()
            .map_err(|e| PdfError::Parse(format!("invalid f64 '{s}': {e}")))
    }
}

/// `bool` 的转换器——"true"/"false"。
struct BoolConverter;

impl PdfConverter<bool> for BoolConverter {
    fn to_pdf_string(&self, value: &bool) -> Result<String> {
        Ok(if *value { "true" } else { "false" }.to_owned())
    }

    fn from_pdf_string(&self, s: &str) -> Result<bool> {
        match s {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(PdfError::Parse(format!("invalid bool '{s}'"))),
        }
    }
}

/// `chrono::DateTime<chrono::Utc>` 的转换器——RFC 3339 格式。
struct ChronoUtcConverter;

impl PdfConverter<chrono::DateTime<chrono::Utc>> for ChronoUtcConverter {
    fn to_pdf_string(&self, value: &chrono::DateTime<chrono::Utc>) -> Result<String> {
        Ok(value.to_rfc3339())
    }

    fn from_pdf_string(&self, s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| PdfError::Parse(format!("invalid RFC 3339 datetime '{s}': {e}")))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant, clippy::items_after_statements)]
    use super::*;

    #[test]
    fn empty_registry_reports_no_converter() {
        let registry = ConverterRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(!registry.has::<String>());
        assert!(registry.to_pdf_string(&42_i64).is_err());
        assert!(registry.from_pdf_string::<i64>("42").is_err());
    }

    #[test]
    fn register_and_roundtrip_string() {
        let mut registry = ConverterRegistry::new();
        registry.register::<String>(Box::new(StringConverter));
        assert!(registry.has::<String>());
        assert_eq!(registry.len(), 1);

        let s = registry.to_pdf_string(&"hello".to_owned()).unwrap();
        assert_eq!(s, "hello");

        let v: String = registry.from_pdf_string("world").unwrap();
        assert_eq!(v, "world");
    }

    #[test]
    fn register_and_roundtrip_i64() {
        let mut registry = ConverterRegistry::new();
        registry.register::<i64>(Box::new(I64Converter));

        assert_eq!(registry.to_pdf_string(&42_i64).unwrap(), "42");
        assert_eq!(registry.to_pdf_string(&-7_i64).unwrap(), "-7");
        let v: i64 = registry.from_pdf_string("99").unwrap();
        assert_eq!(v, 99);
        assert!(registry.from_pdf_string::<i64>("abc").is_err());
    }

    #[test]
    fn register_and_roundtrip_f64() {
        let mut registry = ConverterRegistry::new();
        registry.register::<f64>(Box::new(F64Converter));

        let s = registry.to_pdf_string(&3.14_f64).unwrap();
        assert!(s.contains("3.14"));
        let v: f64 = registry.from_pdf_string("2.718").unwrap();
        assert!((v - 2.718).abs() < 0.001);
    }

    #[test]
    fn register_and_roundtrip_bool() {
        let mut registry = ConverterRegistry::new();
        registry.register::<bool>(Box::new(BoolConverter));

        assert_eq!(registry.to_pdf_string(&true).unwrap(), "true");
        assert_eq!(registry.to_pdf_string(&false).unwrap(), "false");
        assert!(registry.from_pdf_string::<bool>("true").unwrap());
        assert!(!registry.from_pdf_string::<bool>("false").unwrap());
        assert!(!registry.from_pdf_string::<bool>("0").unwrap());
        assert!(registry.from_pdf_string::<bool>("1").unwrap());
        assert!(!registry.from_pdf_string::<bool>("no").unwrap());
        assert!(registry.from_pdf_string::<bool>("yes").unwrap());
        assert!(registry.from_pdf_string::<bool>("maybe").is_err());
    }

    #[test]
    fn register_and_roundtrip_chrono_utc() {
        let mut registry = ConverterRegistry::new();
        registry.register::<chrono::DateTime<chrono::Utc>>(Box::new(ChronoUtcConverter));

        let input = "2024-06-15T12:30:00+00:00";
        let dt: chrono::DateTime<chrono::Utc> = registry.from_pdf_string(input).unwrap();
        let output = registry.to_pdf_string(&dt).unwrap();
        let reparsed: chrono::DateTime<chrono::Utc> = registry.from_pdf_string(&output).unwrap();
        assert_eq!(dt, reparsed);
    }

    #[test]
    fn chrono_converter_rejects_invalid_input() {
        let mut registry = ConverterRegistry::new();
        registry.register::<chrono::DateTime<chrono::Utc>>(Box::new(ChronoUtcConverter));
        assert!(
            registry
                .from_pdf_string::<chrono::DateTime<chrono::Utc>>("not-a-date")
                .is_err()
        );
    }

    #[test]
    fn with_defaults_populates_common_types() {
        let registry = ConverterRegistry::with_defaults();
        assert!(registry.has::<String>());
        assert!(registry.has::<i64>());
        assert!(registry.has::<f64>());
        assert!(registry.has::<bool>());
        assert!(registry.has::<chrono::DateTime<chrono::Utc>>());
        assert_eq!(registry.len(), 5);
    }

    #[test]
    fn default_is_same_as_with_defaults() {
        let registry = ConverterRegistry::default();
        assert_eq!(registry.len(), 5);
    }

    #[test]
    fn register_replaces_existing_converter() {
        let mut registry = ConverterRegistry::new();
        registry.register::<i64>(Box::new(I64Converter));
        assert_eq!(registry.to_pdf_string(&10_i64).unwrap(), "10");

        // Replace with a converter that doubles the value.
        struct DoubleConverter;
        impl PdfConverter<i64> for DoubleConverter {
            fn to_pdf_string(&self, value: &i64) -> Result<String> {
                Ok((value * 2).to_string())
            }
            fn from_pdf_string(&self, s: &str) -> Result<i64> {
                let v: i64 = s
                    .parse()
                    .map_err(|e: std::num::ParseIntError| PdfError::Parse(e.to_string()))?;
                Ok(v / 2)
            }
        }
        registry.register::<i64>(Box::new(DoubleConverter));
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.to_pdf_string(&10_i64).unwrap(), "20");
    }

    #[test]
    fn custom_converter_works() {
        struct UpperCaseConverter;
        impl PdfConverter<String> for UpperCaseConverter {
            fn to_pdf_string(&self, value: &String) -> Result<String> {
                Ok(value.to_uppercase())
            }
            fn from_pdf_string(&self, s: &str) -> Result<String> {
                Ok(s.to_lowercase())
            }
        }

        let mut registry = ConverterRegistry::new();
        registry.register::<String>(Box::new(UpperCaseConverter));

        assert_eq!(
            registry.to_pdf_string(&"hello".to_owned()).unwrap(),
            "HELLO"
        );
        assert_eq!(
            registry.from_pdf_string::<String>("WORLD").unwrap(),
            "world"
        );
    }

    #[test]
    fn error_messages_include_type_name() {
        let registry = ConverterRegistry::new();
        let err = registry.to_pdf_string(&42_i64).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("i64"), "error should mention type name: {msg}");
    }

    #[test]
    fn type_mismatch_error_on_wrong_type() {
        let mut registry = ConverterRegistry::new();
        registry.register::<i64>(Box::new(I64Converter));
        // Trying to use the i64 converter with a String should fail.
        let result = registry.to_pdf_string(&"not a number".to_owned());
        assert!(result.is_err());
    }
}
