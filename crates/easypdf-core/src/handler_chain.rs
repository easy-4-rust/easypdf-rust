//! Priority-based write handler execution chain.
//!
//! Provides [`WriteHandlerChain`] which manages [`PdfWriteHandler`] instances
//! sorted by priority. Handlers with lower priority values execute first.
//!
//! # Priority Constants
//!
//! - [`PRIORITY_HIGH`] (0.0) -- styles, layout, must execute first.
//! - [`PRIORITY_NORMAL`] (10.0) -- default priority for most handlers.
//! - [`PRIORITY_LOW`] (20.0) -- page numbers, watermarks, post-processing.

use crate::error::Result;
use crate::traits::PdfWriteHandler;

/// High priority (0.0) -- styles, layout, and handlers that must run first.
pub const PRIORITY_HIGH: f64 = 0.0;

/// Normal priority (10.0) -- the default for most handlers.
pub const PRIORITY_NORMAL: f64 = 10.0;

/// Low priority (20.0) -- page numbers, watermarks, and post-processing.
pub const PRIORITY_LOW: f64 = 20.0;

/// A [`PdfWriteHandler`] paired with its execution priority.
///
/// Lower priority values execute first. Handlers with equal priority
/// preserve registration order (stable sort).
pub struct WriteHandlerRegistration {
    /// The handler instance.
    handler: Box<dyn PdfWriteHandler>,
    /// Execution priority (lower runs first).
    priority: f64,
}

impl WriteHandlerRegistration {
    /// Create a new registration with the given handler and priority.
    #[must_use]
    pub fn new(handler: Box<dyn PdfWriteHandler>, priority: f64) -> Self {
        Self { handler, priority }
    }

    /// Return the priority of this registration.
    #[must_use]
    pub const fn priority(&self) -> f64 {
        self.priority
    }

    /// Borrow the handler.
    #[must_use]
    pub fn handler(&self) -> &dyn PdfWriteHandler {
        self.handler.as_ref()
    }

    /// Borrow the handler mutably.
    pub fn handler_mut(&mut self) -> &mut dyn PdfWriteHandler {
        self.handler.as_mut()
    }
}

/// An ordered chain of [`PdfWriteHandler`] instances sorted by priority.
///
/// Handlers are invoked in ascending priority order (lowest first) at each
/// lifecycle stage. The chain uses stable sorting so handlers with equal
/// priority preserve their registration order.
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
/// // StyleHandler runs before WatermarkHandler because HIGH < NORMAL.
/// ```
pub struct WriteHandlerChain {
    registrations: Vec<WriteHandlerRegistration>,
    sorted: bool,
}

impl WriteHandlerChain {
    /// Create an empty handler chain.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
            sorted: true,
        }
    }

    /// Register a handler with the given priority.
    ///
    /// The chain will be re-sorted before the next lifecycle call.
    pub fn register(&mut self, handler: Box<dyn PdfWriteHandler>, priority: f64) {
        self.registrations
            .push(WriteHandlerRegistration::new(handler, priority));
        self.sorted = false;
    }

    /// Return the number of registered handlers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    /// Return whether the chain has no handlers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }

    /// Ensure registrations are sorted by priority (ascending, stable).
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

    /// Invoke `before_document` on all handlers in priority order.
    ///
    /// # Errors
    ///
    /// Returns the first error from any handler; subsequent handlers are skipped.
    pub fn before_document(&mut self) -> Result<()> {
        self.ensure_sorted();
        for reg in &mut self.registrations {
            reg.handler.before_document()?;
        }
        Ok(())
    }

    /// Invoke `before_page` on all handlers in priority order.
    ///
    /// # Errors
    ///
    /// Returns the first error from any handler; subsequent handlers are skipped.
    pub fn before_page(&mut self, page_number: usize) -> Result<()> {
        self.ensure_sorted();
        for reg in &mut self.registrations {
            reg.handler.before_page(page_number)?;
        }
        Ok(())
    }

    /// Invoke `after_page` on all handlers in priority order.
    ///
    /// # Errors
    ///
    /// Returns the first error from any handler; subsequent handlers are skipped.
    pub fn after_page(&mut self, page_number: usize) -> Result<()> {
        self.ensure_sorted();
        for reg in &mut self.registrations {
            reg.handler.after_page(page_number)?;
        }
        Ok(())
    }

    /// Invoke `after_document` on all handlers in priority order.
    ///
    /// # Errors
    ///
    /// Returns the first error from any handler; subsequent handlers are skipped.
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
