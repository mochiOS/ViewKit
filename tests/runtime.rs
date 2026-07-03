use viewkit::components::ButtonColor;
use viewkit::layout::{StackAlignment, StackDistribution, StackGap};
use viewkit::runtime::{
    ActionId, ActionQueue, ButtonNode, ComponentInstanceId, NodeId, PaddingNode, RuntimeAction,
    RuntimeEvent, TextNode, TreeBuilderError, VStackNode, ViewNode, ViewNodeKind, ViewRuntime,
    ViewTreeBuilder,
};
use viewkit::theme::Color;
use viewkit::typography::TextAlignment;

const COMPONENT_ID: ComponentInstanceId = ComponentInstanceId(1);

const ROOT_ID: NodeId = NodeId(100);

const PADDING_ID: NodeId = NodeId(101);

const STACK_ID: NodeId = NodeId(102);

const TEXT_ID: NodeId = NodeId(103);

const BUTTON_ID: NodeId = NodeId(104);

const INCREMENT_ACTION_ID: ActionId = ActionId(200);

fn build_counter_tree(counter: i64) -> ViewNode {
    let mut builder = ViewTreeBuilder::new(COMPONENT_ID);

    builder.begin(ViewNode::new(ROOT_ID, ViewNodeKind::Root));

    builder.begin(ViewNode::new(
        PADDING_ID,
        ViewNodeKind::Padding(PaddingNode {
            top: 24.0,
            right: 24.0,
            bottom: 24.0,
            left: 24.0,
        }),
    ));

    builder.begin(ViewNode::new(
        STACK_ID,
        ViewNodeKind::VStack(VStackNode {
            gap: StackGap::Large,

            alignment: StackAlignment::Center,

            distribution: StackDistribution::Center,
        }),
    ));

    builder.leaf(ViewNode::new(
        TEXT_ID,
        ViewNodeKind::Text(TextNode {
            content: format!("count: {counter}"),

            font_size: 18.0,

            line_height: 28.0,

            weight: 600,

            alignment: TextAlignment::Center,

            color: Color::BLACK,
        }),
    ));

    builder.leaf(ViewNode::new(
        BUTTON_ID,
        ViewNodeKind::Button(ButtonNode {
            title: String::from("increment"),

            color: ButtonColor::Accent,

            radius: 0.5,

            action: Some(INCREMENT_ACTION_ID),
        }),
    ));

    builder.end().expect("VStackを閉じられること");

    builder.end().expect("Paddingを閉じられること");

    builder.end().expect("Rootを閉じられること");

    builder.finish().expect("完全なViewツリーを構築できること")
}

#[test]
fn builder_constructs_nested_tree() {
    let tree = build_counter_tree(3);

    assert_eq!(tree.id, ROOT_ID,);

    assert!(matches!(&tree.kind, ViewNodeKind::Root),);

    assert_eq!(tree.children.len(), 1,);

    let padding = &tree.children[0];

    assert_eq!(padding.id, PADDING_ID,);

    assert!(matches!(&padding.kind, ViewNodeKind::Padding(_)),);

    assert_eq!(padding.children.len(), 1,);

    let stack = &padding.children[0];

    assert_eq!(stack.id, STACK_ID,);

    assert!(matches!(&stack.kind, ViewNodeKind::VStack(_)),);

    assert_eq!(stack.children.len(), 2,);

    let text = &stack.children[0];

    assert_eq!(text.id, TEXT_ID,);

    match &text.kind {
        ViewNodeKind::Text(properties) => {
            assert_eq!(properties.content, "count: 3",);

            assert_eq!(properties.font_size, 18.0,);

            assert_eq!(properties.line_height, 28.0,);

            assert_eq!(properties.weight, 600,);

            assert_eq!(properties.alignment, TextAlignment::Center,);

            assert_eq!(properties.color, Color::BLACK,);
        }

        other => {
            panic!(
                "TextNodeを期待しました: \
                 {other:?}"
            );
        }
    }

    let button = &stack.children[1];

    assert_eq!(button.id, BUTTON_ID,);

    match &button.kind {
        ViewNodeKind::Button(properties) => {
            assert_eq!(properties.title, "increment",);

            assert_eq!(properties.color, ButtonColor::Accent,);

            assert_eq!(properties.radius, 0.5,);

            assert_eq!(properties.action, Some(INCREMENT_ACTION_ID,),);
        }

        other => {
            panic!(
                "ButtonNodeを期待しました: \
                 {other:?}"
            );
        }
    }
}

#[test]
fn builder_exposes_component_instance() {
    let builder = ViewTreeBuilder::new(COMPONENT_ID);

    assert_eq!(builder.component_instance(), COMPONENT_ID,);
}

#[test]
fn builder_reports_missing_root() {
    let builder = ViewTreeBuilder::new(COMPONENT_ID);

    let error = builder.finish().expect_err("ルートがないため失敗すること");

    assert_eq!(error, TreeBuilderError::MissingRoot,);
}

#[test]
fn builder_reports_unclosed_nodes() {
    let mut builder = ViewTreeBuilder::new(COMPONENT_ID);

    builder.begin(ViewNode::new(ROOT_ID, ViewNodeKind::Root));

    let error = builder
        .finish()
        .expect_err("閉じていないNodeがあるため失敗すること");

    assert_eq!(error, TreeBuilderError::UnclosedNodes,);
}

#[test]
fn builder_reports_multiple_roots() {
    let mut builder = ViewTreeBuilder::new(COMPONENT_ID);

    builder.leaf(ViewNode::new(NodeId(1), ViewNodeKind::Root));

    builder.leaf(ViewNode::new(NodeId(2), ViewNodeKind::Root));

    let error = builder
        .finish()
        .expect_err("複数のルートがあるため失敗すること");

    assert_eq!(error, TreeBuilderError::MultipleRoots,);
}

#[test]
fn builder_reports_end_without_open_node() {
    let mut builder = ViewTreeBuilder::new(COMPONENT_ID);

    let result = builder.end();

    assert_eq!(result, Err(TreeBuilderError::NoOpenNode,),);
}

#[test]
fn action_queue_starts_empty() {
    let queue = ActionQueue::default();

    assert!(queue.is_empty(),);
}

#[test]
fn action_queue_preserves_fifo_order() {
    let mut queue = ActionQueue::default();

    queue.push(RuntimeAction {
        component_instance: ComponentInstanceId(1),

        node_id: NodeId(10),

        action_id: ActionId(100),

        event: RuntimeEvent::ButtonClicked,
    });

    queue.push(RuntimeAction {
        component_instance: ComponentInstanceId(2),

        node_id: NodeId(20),

        action_id: ActionId(200),

        event: RuntimeEvent::ButtonClicked,
    });

    assert!(!queue.is_empty(),);

    let first = queue.poll().expect("最初のActionが存在すること");

    assert_eq!(first.component_instance, ComponentInstanceId(1),);

    assert_eq!(first.node_id, NodeId(10),);

    assert_eq!(first.action_id, ActionId(100),);

    assert!(matches!(first.event, RuntimeEvent::ButtonClicked),);

    let second = queue.poll().expect("2番目のActionが存在すること");

    assert_eq!(second.component_instance, ComponentInstanceId(2),);

    assert_eq!(second.node_id, NodeId(20),);

    assert_eq!(second.action_id, ActionId(200),);

    assert!(matches!(second.event, RuntimeEvent::ButtonClicked),);

    assert!(queue.poll().is_none(),);

    assert!(queue.is_empty(),);
}

#[test]
fn runtime_starts_without_tree() {
    #[allow(unused_mut)]
    let mut runtime = ViewRuntime::new(COMPONENT_ID);

    assert!(runtime.tree().is_none(),);
}

#[test]
fn runtime_commits_tree() {
    let mut runtime = ViewRuntime::new(COMPONENT_ID);

    runtime.commit(build_counter_tree(0));

    let tree = runtime.tree().expect("commit後にツリーが存在すること");

    assert_eq!(tree.id, ROOT_ID,);

    assert_eq!(tree.children.len(), 1,);
}

#[test]
fn runtime_replaces_committed_tree() {
    let mut runtime = ViewRuntime::new(COMPONENT_ID);

    runtime.commit(build_counter_tree(0));

    runtime.commit(build_counter_tree(1));

    let tree = runtime.tree().expect("更新後にツリーが存在すること");

    let text = &tree.children[0].children[0].children[0];

    match &text.kind {
        ViewNodeKind::Text(properties) => {
            assert_eq!(properties.content, "count: 1",);
        }

        other => {
            panic!(
                "TextNodeを期待しました: \
                 {other:?}"
            );
        }
    }
}

#[test]
fn node_ids_remain_stable_after_rebuild() {
    let first = build_counter_tree(0);

    let second = build_counter_tree(1);

    assert_eq!(first.id, second.id,);

    let first_padding = &first.children[0];

    let second_padding = &second.children[0];

    assert_eq!(first_padding.id, second_padding.id,);

    let first_stack = &first_padding.children[0];

    let second_stack = &second_padding.children[0];

    assert_eq!(first_stack.id, second_stack.id,);

    assert_eq!(first_stack.children[0].id, second_stack.children[0].id,);

    assert_eq!(first_stack.children[1].id, second_stack.children[1].id,);

    assert_eq!(first_stack.children[0].id, TEXT_ID,);

    assert_eq!(first_stack.children[1].id, BUTTON_ID,);
}

#[test]
fn runtime_builds_view_from_committed_tree() {
    let mut runtime = ViewRuntime::new(COMPONENT_ID);

    runtime.commit(build_counter_tree(0));

    let view = runtime.build_view();

    assert!(view.is_some(), "ViewNodeからViewを構築できること",);
}

#[test]
fn runtime_does_not_build_view_without_tree() {
    let mut runtime = ViewRuntime::new(COMPONENT_ID);

    assert!(runtime.build_view().is_none(),);
}

#[test]
fn runtime_has_no_action_before_interaction() {
    let mut runtime = ViewRuntime::new(COMPONENT_ID);

    runtime.commit(build_counter_tree(0));

    runtime.collect_actions();

    assert!(
        runtime.poll_action().is_none(),
        "Buttonがクリックされる前はActionを生成しない",
    );
}
