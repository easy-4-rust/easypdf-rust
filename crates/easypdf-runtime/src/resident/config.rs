//! Resident 守护进程配置。

use std::time::Duration;

/// Resident PDF 守护进程的配置。
///
/// 控制空闲超时、会话上限、自动保存行为和 socket 权限。
/// 使用 [`Default`] 获取合理的默认值，或手动构建。
///
/// # 示例
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
    /// 空闲超时：服务器在无活动达到此时间后自动关闭。
    ///
    /// 默认为 5 分钟。
    pub idle_timeout: Duration,

    /// 最大并发文档会话数。
    ///
    /// 默认为 16。
    pub max_sessions: usize,

    /// 脏会话的自动保存模式。
    ///
    /// 默认为 [`AutosaveMode::Adaptive`]。
    pub autosave: AutosaveMode,

    /// Unix socket 文件权限模式（例如 `0o600`）。
    ///
    /// 仅在 Unix 平台上生效。默认为 `0o600`（仅所有者可读写）。
    #[cfg(unix)]
    pub socket_mode: u32,
}

/// 脏文档会话的自动保存策略。
///
/// 借鉴自 `OfficeCLI` 模式：自适应自动保存使用保存耗时的
/// 指数移动平均值（EMA）动态调整保存间隔，防止后台保存
/// 占用超过约 25% 的挂钟时间。
#[derive(Debug, Clone)]
pub enum AutosaveMode {
    /// 禁用自动保存。脏会话仅在收到显式 `Save` 命令时保存。
    Disabled,
    /// 固定自动保存间隔。
    Fixed(Duration),
    /// 自适应自动保存（默认）。
    ///
    /// 间隔根据测量到的保存耗时动态调整：
    /// `clamp(4 * EMA(save_duration), min_interval, max_interval)`。
    Adaptive {
        /// 最小自动保存间隔（下限）。
        min_interval: Duration,
        /// 最大自动保存间隔（上限）。
        max_interval: Duration,
        /// 任何保存测量之前的初始间隔。
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
    /// 根据新的保存耗时样本计算下一个自适应间隔。
    ///
    /// 使用 alpha = 0.3 的 EMA（指数移动平均）。
    /// 如果不处于自适应模式，返回 `None`。
    ///
    /// # 参数
    ///
    /// * `prev_ema_secs` - 上一次的 EMA 值（秒），首次采样时为 `None`。
    /// * `save_duration` - 本次保存耗时。
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

    /// 返回自适应模式的初始间隔，其他模式返回 `None`。
    #[must_use]
    pub fn initial_interval(&self) -> Option<Duration> {
        match self {
            Self::Adaptive { initial, .. } => Some(*initial),
            Self::Fixed(d) => Some(*d),
            Self::Disabled => None,
        }
    }
}
