//! 候选窗口的输出端。

use glimmer_platform::CandidateRenderer;
use glimmer_platform::protocol::{Frame, ScreenRect};

/// 候选窗口 / 状态条的画法：由谁画（`[general] renderer`）、用什么字体（`[general] font`，空为系统字体）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSettings {
    pub renderer: CandidateRenderer,

    pub font: String,
}

/// Router 只产出帧，画交给它；Windows 上由 UI 线程实现。
pub trait CandidateSink: Send {
    /// 把候选窗口摆到 `rect`（组句范围的屏幕矩形）下方并按 `frame` 重绘。
    fn show(&self, frame: Frame, rect: ScreenRect);

    fn hide(&self);

    /// 换画法：装上时与配置热加载后调，只在设置变了时调。
    fn configure(&self, settings: RenderSettings);
}

/// 不画候选窗口的空实现。
pub struct NoopSink;

impl CandidateSink for NoopSink {
    fn show(&self, _frame: Frame, _rect: ScreenRect) {}

    fn hide(&self) {}

    fn configure(&self, _settings: RenderSettings) {}
}
