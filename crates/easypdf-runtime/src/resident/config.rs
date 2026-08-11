//! Configuration for the resident daemon.

use std::time::Duration;

/// Configuration for the resident PDF daemon.
///
/// Controls idle timeout, session limits, autosave behavior, and socket
/// permissions. Use [`Default`] for sensible defaults, or build manually.
///
/// # Examples
///
/// ```
/// use easypdf_runtime::resident::ResidentConfig;
/// use std::time::Duration;
///
/// let config = ResidentConfig {
///     idle_timeout: Duration::from_secs(600),
///     max_sessions: 8,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ResidentConfig {
    /// Idle timeout: the server shuts down after this duration with no activity.
    ///
    /// Defaults to 5 minutes.
    pub idle_timeout: Duration,

    /// Maximum number of concurrent document sessions.
    ///
    /// Defaults to 16.
    pub max_sessions: usize,

    /// Autosave mode for dirty sessions.
    ///
    /// Defaults to [`AutosaveMode::Adaptive`].
    pub autosave: AutosaveMode,

    /// Unix socket file permission mode (e.g. `0o600`).
    ///
    /// Only applies on Unix platforms. Defaults to `0o600` (owner read/write only).
    #[cfg(unix)]
    pub socket_mode: u32,
}

/// Autosave strategy for dirty document sessions.
///
/// Borrowed from the `OfficeCLI` pattern: adaptive autosave uses an
/// exponential moving average (EMA) of save durations to dynamically
/// adjust the save interval, preventing background saves from consuming
/// more than ~25% of wall-clock time.
#[derive(Debug, Clone)]
pub enum AutosaveMode {
    /// Autosave disabled. Dirty sessions are only saved on explicit `Save` command.
    Disabled,
    /// Fixed autosave interval.
    Fixed(Duration),
    /// Adaptive autosave (default).
    ///
    /// The interval adjusts based on measured save durations:
    /// `clamp(4 * EMA(save_duration), min_interval, max_interval)`.
    Adaptive {
        /// Minimum autosave interval (floor).
        min_interval: Duration,
        /// Maximum autosave interval (ceiling).
        max_interval: Duration,
        /// Initial interval before any save measurements.
        initial: Duration,
    },
}

impl Default for ResidentConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(300),
            max_sessions: 16,
            autosave: AutosaveMode::Adaptive {
                min_interval: Duration::from_secs(10),
                max_interval: Duration::from_secs(300),
                initial: Duration::from_secs(60),
            },
            #[cfg(unix)]
            socket_mode: 0o600,
        }
    }
}

impl AutosaveMode {
    /// Compute the next adaptive interval given a new save duration sample.
    ///
    /// Uses EMA (exponential moving average) with alpha = 0.3.
    /// Returns `None` if not in adaptive mode.
    #[must_use]
    pub fn next_adaptive_interval(
        &self,
        prev_ema_secs: Option<f64>,
        save_duration: Duration,
    ) -> Option<Duration> {
        match self {
            Self::Adaptive {
                min_interval,
                max_interval,
                ..
            } => {
                const ALPHA: f64 = 0.3;
                const MULTIPLIER: f64 = 4.0;

                let sample = save_duration.as_secs_f64();
                let ema = match prev_ema_secs {
                    Some(prev) => ALPHA * sample + (1.0 - ALPHA) * prev,
                    None => sample,
                };

                let interval_secs = MULTIPLIER * ema;
                let clamped = interval_secs
                    .max(min_interval.as_secs_f64())
                    .min(max_interval.as_secs_f64());
                Some(Duration::from_secs_f64(clamped))
            }
            Self::Disabled | Self::Fixed(_) => None,
        }
    }

    /// Returns the initial interval for adaptive mode, or `None` otherwise.
    #[must_use]
    pub fn initial_interval(&self) -> Option<Duration> {
        match self {
            Self::Adaptive { initial, .. } => Some(*initial),
            Self::Fixed(d) => Some(*d),
            Self::Disabled => None,
        }
    }
}
