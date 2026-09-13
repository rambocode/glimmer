//! 弹出按钮子类：长得和其他设置项一样，但点它不弹菜单，弹的是带搜索框的字体列表。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSPopUpButton};
use objc2_foundation::NSRect;

use super::source::FontListSource;

/// 视图状态。
pub(super) struct ButtonState {
    /// 列表与下拉框的主人。
    pub source: Retained<FontListSource>,
}

define_class!(
    // SAFETY: NSPopUpButton 允许子类化；没有实现 Drop。
    #[unsafe(super(NSPopUpButton))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ButtonState]
    pub(super) struct FontPopUpButton;

    impl FontPopUpButton {
        /// 鼠标点下去：不走菜单，开我们的下拉框。
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            self.ivars().source.open();
        }

        /// 键盘（空格 / 回车）触发也一样。
        #[unsafe(method(performClick:))]
        fn perform_click(&self, _sender: Option<&AnyObject>) {
            self.ivars().source.open();
        }
    }
);

impl FontPopUpButton {
    pub(super) fn new(mtm: MainThreadMarker, source: Retained<FontListSource>) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(ButtonState { source });
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO, pullsDown: false] }
    }
}
