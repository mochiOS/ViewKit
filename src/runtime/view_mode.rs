//! Rustで宣言されたViewをランタイムのViewツリーへ変換します。

use super::{ComponentInstanceId, NodeId, ViewNode};

/// Viewツリーの構築中に使用されるコンテキストです。
///
/// ViewKitランタイムは、このコンテキストを使用して各Viewへ
/// 一意な[`NodeId`]を割り当てます。
#[derive(Debug)]
pub struct ViewNodeContext {
    component_instance: ComponentInstanceId,
    next_node_id: u64,
}

impl ViewNodeContext {
    /// 指定されたコンポーネントインスタンス用の
    /// Viewツリー構築コンテキストを作成します。
    pub(crate) const fn new(component_instance: ComponentInstanceId) -> Self {
        Self {
            component_instance,
            next_node_id: 0,
        }
    }

    /// 新しいNodeIdを割り当てます。
    pub(crate) fn allocate_node_id(&mut self) -> NodeId {
        let local_id = self.next_node_id;

        self.next_node_id = self.next_node_id.wrapping_add(1);

        NodeId(
            self.component_instance
                .0
                .wrapping_mul(0x1_0000_0000)
                .wrapping_add(local_id),
        )
    }
}

/// Rustで宣言されたViewをランタイムの[`ViewNode`]へ変換するトレイトです。
///
/// ViewKitの各Viewはこのトレイトを実装し、Viewツリーの再構築時に
/// ランタイムが保持できる中間表現へ変換されます。
///
/// 通常、アプリケーション側でこのメソッドを直接呼び出す必要はありません。
pub trait IntoViewNode {
    /// このViewをランタイムのViewNodeへ変換します。
    fn into_view_node(self, context: &mut ViewNodeContext) -> ViewNode;
}

impl IntoViewNode for ViewNode {
    fn into_view_node(self, _context: &mut ViewNodeContext) -> ViewNode {
        self
    }
}

impl<T> IntoViewNode for Box<T>
where
    T: IntoViewNode + ?Sized,
{
    fn into_view_node(self, context: &mut ViewNodeContext) -> ViewNode {
        (*self).into_view_node(context)
    }
}
