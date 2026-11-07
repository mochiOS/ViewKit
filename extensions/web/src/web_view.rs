use crate::{WebError, WebPage};
use std::rc::Rc;
use viewkit::{
    event::{EventContext, EventResult, ViewEvent},
    geometry::{Rect, Size},
    view::{Constraints, MeasureContext, PaintContext, View},
};

/// Webページを配置する通常のViewKit Viewです。
///
/// ページの実体はWebPageが保持します。本Viewは背景・枠・ツールバーを追加せず、
/// 完成した表示をバックエンドのViewへ委譲します。measure/paintからエンジンを
/// 進めません。入力・IME・アクセシビリティは接続先のViewが統合する契約です。
/// 現在は実際のネイティブアダプター未接続のため、公開経路から作成できません。
/// 一度作成したViewをCloneしてViewツリーへ配置します。Cloneは同じ表示を保持し、
/// 新しいページや表示を作りません。同時に複数ウィンドウへ配置しないでください。
/// ```rust,compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<viewkit_web::WebView>();
/// ```
/// @since Unreleased
#[derive(Clone)]
pub struct WebView {
    inner: Rc<Presentation>,
}
struct Presentation {
    page: WebPage,
    content: Rc<dyn View>,
}
impl WebView {
    /// ページの表示オブジェクトを作成します。URLの読み込みは開始しません。
    ///
    /// ## Parameters
    /// - `page`: アプリが保持しているページへの共有参照です。
    /// ## Returns
    /// Cloneして同じ配置へ保持できる表示オブジェクトを返します。
    /// ## Errors
    /// 閉じたページ、停止したセッション、同一ページの既存表示をWebErrorで返します。
    /// ## Example
    /// ```rust,no_run
    /// # fn example(page: viewkit_web::WebPage) -> Result<(), viewkit_web::WebError> {
    /// let view = viewkit_web::WebView::new(page.clone())?;
    /// # Ok(()) }
    /// ```
    /// @since Unreleased
    pub fn new(page: WebPage) -> Result<Self, WebError> {
        page.inner.check()?;
        if page.inner.presented.replace(true) {
            return Err(WebError::AlreadyPresented);
        }
        let content = page.inner.driver.view();
        Ok(Self {
            inner: Rc::new(Presentation { page, content }),
        })
    }
    /// 表示しているページへの共有参照を返します。ページは複製しません。
    ///
    /// ## Returns
    /// 同一ページを参照するWebPageです。
    /// @since Unreleased
    pub fn page(&self) -> WebPage {
        self.inner.page.clone()
    }
}
impl View for WebView {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.inner.content.measure(constraints, context)
    }
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if self.inner.page.inner.check().is_ok() {
            self.inner.content.paint(bounds, context);
        }
    }
    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if self.inner.page.inner.check().is_ok() {
            self.inner.content.handle_event(bounds, event, context)
        } else {
            EventResult::Ignored
        }
    }
}
impl Drop for Presentation {
    fn drop(&mut self) {
        self.page.inner.presented.set(false);
    }
}
