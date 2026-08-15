//! 基于优先级的写入处理程序执行链。
//!
//! 提供 [`WriteHandlerChain`]，管理按优先级排序的 [`PdfWriteHandler`]
//! 实例。优先级值越小的处理程序越先执行。
//!
//! # 优先级常量
//!
//! - [`PRIORITY_HIGH`]（0.0）——样式、布局，必须最先执行。
//! - [`PRIORITY_NORMAL`]（10.0）——大多数处理程序的默认优先级。
//! - [`PRIORITY_LOW`]（20.0）——页码、水印、后处理。

use crate::error::Result;
use crate::traits::PdfWriteHandler;

/// 高优先级（0.0）——样式、布局和必须最先运行的处理程序。
pub const PRIORITY_HIGH: f64 = 0.0;

/// 普通优先级（10.0）——大多数处理程序的默认值。
pub const PRIORITY_NORMAL: f64 = 10.0;

/// 低优先级（20.0）——页码、水印和后处理。
pub const PRIORITY_LOW: f64 = 20.0;

/// 与其执行优先级配对的 [`PdfWriteHandler`]。
///
/// 优先级值越小越先执行。优先级相同的处理程序
/// 保持注册顺序（稳定排序）。
pub struct WriteHandlerRegistration {
    /// 处理程序实例。
    handler: Box<dyn PdfWriteHandler>,
    /// 执行优先级（值越小越先执行）。
    priority: f64,
}

impl WriteHandlerRegistration {
    /// 使用给定的处理程序和优先级创建新的注册。
    #[must_use]
    pub fn new(handler: Box<dyn PdfWriteHandler>, priority: f64) -> Self {
        Self { handler, priority }
    }

    /// 返回此注册的优先级。
    #[must_use]
    pub const fn priority(&self) -> f64 {
        self.priority
    }

    /// 借用处理程序。
    #[must_use]
    pub fn handler(&self) -> &dyn PdfWriteHandler {
        self.handler.as_ref()
    }

    /// 可变借用处理程序。
    pub fn handler_mut(&mut self) -> &mut dyn PdfWriteHandler {
        self.handler.as_mut()
    }
}

/// 按优先级排序的 [`PdfWriteHandler`] 实例有序链。
///
/// 在每个生命周期阶段，处理程序按优先级升序（最小的最先）调用。
/// 链使用稳定排序，因此优先级相同的处理程序保持注册顺序。
///
/// # Examples
///
/// ```
/// use easypdf_core::handler_chain::{WriteHandlerChain, PRIORITY_HIGH, PRIORITY_NORMAL};
/// use easypdf_core::{PdfWriteHandler, Result};
///
/// struct StyleHandler;
/// impl PdfWriteHandler for StyleHandler {}
///
/// struct WatermarkHandler;
/// impl PdfWriteHandler for WatermarkHandler {}
///
/// let mut chain = WriteHandlerChain::new();
/// chain.register(Box::new(WatermarkHandler), PRIORITY_NORMAL);
/// chain.register(Box::new(StyleHandler), PRIORITY_HIGH);
/// // StyleHandler 在 WatermarkHandler 之前运行，因为 HIGH < NORMAL。
/// ```
pub struct WriteHandlerChain {
    registrations: Vec<WriteHandlerRegistration>,
    sorted: bool,
}

impl WriteHandlerChain {
    /// 创建空的处理程序链。
    #[must_use]
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
            sorted: true,
        }
    }

    /// 使用给定优先级注册处理程序。
    ///
    /// 链将在下次生命周期调用前重新排序。
    pub fn register(&mut self, handler: Box<dyn PdfWriteHandler>, priority: f64) {
        self.registrations
            .push(WriteHandlerRegistration::new(handler, priority));
        self.sorted = false;
    }

    /// 返回已注册处理程序的数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    /// 返回链是否没有处理程序。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }

    /// 确保注册按优先级排序（升序、稳定）。
    fn ensure_sorted(&mut self) {
        if !self.sorted {
            self.registrations.sort_by(|a, b| {
                a.priority
                    .partial_cmp(&b.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.sorted = true;
        }
    }

    /// 按优先级顺序在所有处理程序上调用 `before_document`。
    ///
    /// # Errors
    ///
    /// 返回任何处理程序的第一个错误；后续处理程序被跳过。
    pub fn before_document(&mut self) -> Result<()> {
        self.ensure_sorted();
        for reg in &mut self.registrations {
            reg.handler.before_document()?;
        }
        Ok(())
    }

    /// 按优先级顺序在所有处理程序上调用 `before_page`。
    ///
    /// # Errors
    ///
    /// 返回任何处理程序的第一个错误；后续处理程序被跳过。
    pub fn before_page(&mut self, page_number: usize) -> Result<()> {
        self.ensure_sorted();
        for reg in &mut self.registrations {
            reg.handler.before_page(page_number)?;
        }
        Ok(())
    }

    /// 按优先级顺序在所有处理程序上调用 `after_page`。
    ///
    /// # Errors
    ///
    /// 返回任何处理程序的第一个错误；后续处理程序被跳过。
    pub fn after_page(&mut self, page_number: usize) -> Result<()> {
        self.ensure_sorted();
        for reg in &mut self.registrations {
            reg.handler.after_page(page_number)?;
        }
        Ok(())
    }

    /// 按优先级顺序在所有处理程序上调用 `after_document`。
    ///
    /// # Errors
    ///
    /// 返回任何处理程序的第一个错误；后续处理程序被跳过。
    pub fn after_document(&mut self) -> Result<()> {
        self.ensure_sorted();
        for reg in &mut self.registrations {
            reg.handler.after_document()?;
        }
        Ok(())
    }
}

impl Default for WriteHandlerChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Records the order in which handlers are invoked.
    #[derive(Clone)]
    struct RecordingHandler {
        name: &'static str,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl PdfWriteHandler for RecordingHandler {
        fn before_document(&mut self) -> Result<()> {
            self.log.lock().expect("poisoned").push(self.name);
            Ok(())
        }
        fn before_page(&mut self, _page_number: usize) -> Result<()> {
            self.log.lock().expect("poisoned").push(self.name);
            Ok(())
        }
        fn after_page(&mut self, _page_number: usize) -> Result<()> {
            self.log.lock().expect("poisoned").push(self.name);
            Ok(())
        }
        fn after_document(&mut self) -> Result<()> {
            self.log.lock().expect("poisoned").push(self.name);
            Ok(())
        }
    }

    struct FailingHandler;

    impl PdfWriteHandler for FailingHandler {
        fn before_document(&mut self) -> Result<()> {
            Err(crate::error::PdfError::Other("intentional failure".into()))
        }
    }

    #[test]
    fn empty_chain_succeeds() {
        let mut chain = WriteHandlerChain::new();
        assert!(chain.before_document().is_ok());
        assert!(chain.before_page(1).is_ok());
        assert!(chain.after_page(1).is_ok());
        assert!(chain.after_document().is_ok());
        assert!(chain.is_empty());
    }

    #[test]
    fn handlers_execute_in_priority_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut chain = WriteHandlerChain::new();
        chain.register(
            Box::new(RecordingHandler {
                name: "low",
                log: Arc::clone(&log),
            }),
            PRIORITY_LOW,
        );
        chain.register(
            Box::new(RecordingHandler {
                name: "high",
                log: Arc::clone(&log),
            }),
            PRIORITY_HIGH,
        );
        chain.register(
            Box::new(RecordingHandler {
                name: "normal",
                log: Arc::clone(&log),
            }),
            PRIORITY_NORMAL,
        );

        chain.before_document().unwrap();
        let entries: Vec<_> = log.lock().expect("poisoned").clone();
        assert_eq!(entries, vec!["high", "normal", "low"]);
    }

    #[test]
    fn equal_priority_preserves_registration_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut chain = WriteHandlerChain::new();
        chain.register(
            Box::new(RecordingHandler {
                name: "first",
                log: Arc::clone(&log),
            }),
            PRIORITY_NORMAL,
        );
        chain.register(
            Box::new(RecordingHandler {
                name: "second",
                log: Arc::clone(&log),
            }),
            PRIORITY_NORMAL,
        );

        chain.before_page(1).unwrap();
        let entries: Vec<_> = log.lock().expect("poisoned").clone();
        assert_eq!(entries, vec!["first", "second"]);
    }

    #[test]
    fn error_stops_chain_execution() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut chain = WriteHandlerChain::new();
        chain.register(
            Box::new(RecordingHandler {
                name: "before",
                log: Arc::clone(&log),
            }),
            PRIORITY_HIGH,
        );
        chain.register(Box::new(FailingHandler), PRIORITY_NORMAL);
        chain.register(
            Box::new(RecordingHandler {
                name: "after",
                log: Arc::clone(&log),
            }),
            PRIORITY_LOW,
        );

        let result = chain.before_document();
        assert!(result.is_err());
        let entries: Vec<_> = log.lock().expect("poisoned").clone();
        assert_eq!(entries, vec!["before"]);
    }

    #[test]
    fn len_tracks_registrations() {
        let mut chain = WriteHandlerChain::new();
        assert_eq!(chain.len(), 0);
        chain.register(Box::new(FailingHandler), PRIORITY_NORMAL);
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn default_is_empty() {
        let chain = WriteHandlerChain::default();
        assert!(chain.is_empty());
    }

    #[test]
    fn lifecycle_methods_all_invoked() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut chain = WriteHandlerChain::new();
        chain.register(
            Box::new(RecordingHandler {
                name: "h",
                log: Arc::clone(&log),
            }),
            PRIORITY_NORMAL,
        );

        chain.before_document().unwrap();
        chain.before_page(1).unwrap();
        chain.after_page(1).unwrap();
        chain.after_document().unwrap();

        let entries: Vec<_> = log.lock().expect("poisoned").clone();
        assert_eq!(entries, vec!["h", "h", "h", "h"]);
    }
}
