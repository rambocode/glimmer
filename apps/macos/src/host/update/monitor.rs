//! 检查更新 / 下载的轮询定时器：有线程在跑时每 0.2 秒看一次通道；与云服务测试的定时器同一写法，
//! 分开是因为两件事可能同时进行、各自起停。另有两只定时器给自动检查用：一次性的把启动那次推迟到用户打上字之后，
//! 重复的按 `CHECK_INTERVAL` 周期在进程一直开着时再查。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSObject, NSObjectProtocol, NSTimer};

/// 轮询间隔。
const POLL_INTERVAL: f64 = 0.2;

pub struct UpdateMonitor {
    /// 定时器；没在等结果时为 `None`。
    timer: Option<Retained<NSTimer>>,

    /// 启动时自动检查的延迟定时器（一次性）；响过或没排就是 `None`。
    delayed: Option<Retained<NSTimer>>,

    /// 周期自动检查的定时器（重复）；排上就一直在。
    periodic: Option<Retained<NSTimer>>,

    mtm: MainThreadMarker,
}

impl UpdateMonitor {
    pub fn new(mtm: MainThreadMarker) -> Self {
        Self {
            timer: None,
            delayed: None,
            periodic: None,
            mtm,
        }
    }

    /// `delay` 秒后在主线程调一次 `Host::auto_update_check`。已经排了就不重复排。
    pub fn schedule_auto(&mut self, delay: f64) {
        if self.delayed.is_some() {
            return;
        }
        let target = UpdateTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                delay,
                &target,
                sel!(autoCheck:),
                None,
                false,
            )
        };
        self.delayed = Some(timer);
    }

    /// 延迟定时器响了：清掉记录。
    pub fn clear_delayed(&mut self) {
        self.delayed = None;
    }

    /// 每隔 `interval` 秒在主线程调一次 `Host::auto_update_check`（到没到点由它按标记文件判断）。排过就不再排。
    pub fn schedule_periodic(&mut self, interval: f64) {
        if self.periodic.is_some() {
            return;
        }
        let target = UpdateTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                interval,
                &target,
                sel!(autoCheck:),
                None,
                true,
            )
        };
        self.periodic = Some(timer);
    }

    /// 开始轮询；已经在跑就不重复起。
    pub fn start(&mut self) {
        if self.timer.is_some() {
            return;
        }
        let target = UpdateTicker::new(self.mtm);
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                POLL_INTERVAL,
                &target,
                sel!(tick:),
                None,
                true,
            )
        };
        self.timer = Some(timer);
    }

    pub fn stop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.invalidate();
        }
    }
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct UpdateTicker;

    impl UpdateTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| h.poll_update());
        }

        #[unsafe(method(autoCheck:))]
        fn auto_check(&self, _timer: Option<&AnyObject>) {
            crate::host::with(|h| {
                h.update.monitor.clear_delayed();
                h.auto_update_check();
            });
        }
    }

    unsafe impl NSObjectProtocol for UpdateTicker {}
);

impl UpdateTicker {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
