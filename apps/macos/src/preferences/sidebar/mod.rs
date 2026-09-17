//! 设置窗口左侧的导航栏：浅灰底色（`SIDEBAR_COLOR`）上一列「图标 + 页名」，选中哪行右边就显示哪一页。

mod source;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBox, NSBoxType, NSColor, NSFocusRingType, NSScrollView,
    NSTableColumn, NSTableView, NSTableViewSelectionHighlightStyle, NSTableViewStyle,
    NSTitlePosition, NSView,
};
use objc2_foundation::{NSIndexSet, NSPoint, NSRect, NSSize, NSString};

use source::SidebarSource;

/// 侧栏宽度。
pub const SIDEBAR_WIDTH: f64 = 180.0;

/// 侧栏底色 #F9F9FA（sRGB），不随系统深浅色变。
const SIDEBAR_COLOR: (f64, f64, f64) = (
    0xF9 as f64 / 255.0,
    0xF9 as f64 / 255.0,
    0xFA as f64 / 255.0,
);

/// 侧栏顶部留给红黄绿按钮的高度（标题栏透明，侧栏一直伸到窗口顶）。
const TOP_INSET: f64 = 52.0;

/// 每行的高度。
const ROW_HEIGHT: f64 = 30.0;

/// 侧栏的一项：页名与 SF Symbol 名。
pub struct SidebarEntry {
    /// 显示的页名。
    pub title: &'static str,

    /// 图标的 SF Symbol 名。
    pub symbol: &'static str,
}

/// 侧栏：纯色底板里装一个源列表样式的表格。
pub struct Sidebar {
    /// 侧栏本体（无边框、纯色填充的 NSBox），加到窗口内容视图的最左边。
    view: Retained<NSBox>,

    /// 列表。
    table: Retained<NSTableView>,

    /// 数据源与代理，要和侧栏活得一样久。
    _source: Retained<SidebarSource>,
}

impl Sidebar {
    /// `on_select` 在用户点了某一行（或程序调 [`Sidebar::select`]）时收到行号。
    pub fn new(
        mtm: MainThreadMarker,
        entries: Vec<SidebarEntry>,
        height: f64,
        on_select: Box<dyn Fn(usize)>,
    ) -> Self {
        let frame = NSRect::new(NSPoint::ZERO, NSSize::new(SIDEBAR_WIDTH, height));
        // 纯色底板：NSBox 的 Custom 类型可以直接填色、不画边框
        let view = NSBox::initWithFrame(mtm.alloc(), frame);
        view.setBoxType(NSBoxType::Custom);
        view.setTitlePosition(NSTitlePosition::NoTitle);
        view.setContentViewMargins(NSSize::ZERO);
        view.setBorderWidth(0.0);
        let (r, g, b) = SIDEBAR_COLOR;
        view.setFillColor(&NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0));
        view.setAutoresizingMask(NSAutoresizingMaskOptions::ViewHeightSizable);

        let table = NSTableView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        let column = NSTableColumn::initWithIdentifier(mtm.alloc(), &NSString::from_str("page"));
        column.setWidth(SIDEBAR_WIDTH);
        table.addTableColumn(&column);
        table.setHeaderView(None);
        table.setRowHeight(ROW_HEIGHT);
        table.setStyle(NSTableViewStyle::SourceList);
        table.setSelectionHighlightStyle(NSTableViewSelectionHighlightStyle::Regular);
        table.setAllowsEmptySelection(false);
        table.setAllowsMultipleSelection(false);
        table.setBackgroundColor(&NSColor::clearColor());
        table.setFocusRingType(NSFocusRingType::None);
        let source = SidebarSource::new(mtm, entries, on_select);
        // SAFETY: 数据源随 Sidebar 活着；两个协议的方法签名与 define_class 里一致
        unsafe {
            table.setDataSource(Some(ProtocolObject::from_ref(&*source)));
            table.setDelegate(Some(ProtocolObject::from_ref(&*source)));
        }
        table.reloadData();

        // 表格顶部贴着 TOP_INSET，窗口变高时只拉伸下面的空白
        let scroll = NSScrollView::initWithFrame(
            mtm.alloc(),
            NSRect::new(
                NSPoint::ZERO,
                NSSize::new(SIDEBAR_WIDTH, height - TOP_INSET),
            ),
        );
        scroll.setDrawsBackground(false);
        scroll.setHasVerticalScroller(false);
        scroll.setDocumentView(Some(&table));
        scroll.setAutoresizingMask(NSAutoresizingMaskOptions::ViewHeightSizable);
        view.addSubview(&scroll);

        Self {
            view,
            table,
            _source: source,
        }
    }

    /// 侧栏视图，加到窗口内容视图里。
    pub fn view(&self) -> &NSView {
        &self.view
    }

    /// 选中第 `index` 行（会触发 `on_select`）。
    pub fn select(&self, index: usize) {
        let indexes = NSIndexSet::indexSetWithIndex(index);
        self.table
            .selectRowIndexes_byExtendingSelection(&indexes, false);
    }
}
