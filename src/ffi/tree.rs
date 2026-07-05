use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::components::VStack;
use crate::layout::{IntoStackChild, StackAlignment, StackChild, StackGap};
use crate::view::View;

use super::{VK_EVENT_BUTTON_CLICKED, VkActionEvent, VkStatus};

pub(crate) type SharedActionQueue = Rc<RefCell<VecDeque<VkActionEvent>>>;

pub(crate) type FfiViewFactory =
    Box<dyn FnOnce(u64, Vec<FfiBuiltView>, &mut FfiBuildContext) -> Result<FfiBuiltView, VkStatus>>;

pub(crate) enum FfiBuiltView {
    View(Box<dyn View>),
    StackChild(StackChild),
}

impl FfiBuiltView {
    pub(crate) fn into_stack_child(self) -> StackChild {
        match self {
            Self::View(view) => StackChild::new(view),

            Self::StackChild(child) => child,
        }
    }

    pub(crate) fn into_view(self) -> Box<dyn View> {
        match self {
            Self::View(view) => view,

            Self::StackChild(child) => Box::new(
                VStack::new()
                    .gap(StackGap::None)
                    .alignment(StackAlignment::Start)
                    .child(child),
            ),
        }
    }
}

#[derive(Clone)]
pub(crate) struct FfiBuildContext {
    component_instance_id: u64,
    actions: SharedActionQueue,
}

impl FfiBuildContext {
    pub(crate) fn new(component_instance_id: u64, actions: SharedActionQueue) -> Self {
        Self {
            component_instance_id,
            actions,
        }
    }

    pub(crate) fn button_callback(&self, node_id: u64, action_id: u64) -> impl FnMut() + 'static {
        let component_instance_id = self.component_instance_id;

        let actions = Rc::clone(&self.actions);

        move || {
            actions.borrow_mut().push_back(VkActionEvent {
                component_instance_id,
                node_id,
                action_id,
                event_kind: VK_EVENT_BUTTON_CLICKED,
            });
        }
    }
}

enum FfiNodeKind {
    Root,
    Component(FfiViewFactory),
}

pub(crate) struct FfiNode {
    id: u64,
    kind: FfiNodeKind,
    children: Vec<FfiNode>,
}

impl FfiNode {
    pub(crate) fn root(id: u64) -> Self {
        Self {
            id,
            kind: FfiNodeKind::Root,
            children: Vec::new(),
        }
    }

    pub(crate) fn component(id: u64, factory: FfiViewFactory) -> Self {
        Self {
            id,
            kind: FfiNodeKind::Component(factory),
            children: Vec::new(),
        }
    }

    fn build(self, context: &mut FfiBuildContext) -> Result<FfiBuiltView, VkStatus> {
        let FfiNode { id, kind, children } = self;

        let children = children
            .into_iter()
            .map(|child| child.build(context))
            .collect::<Result<Vec<_>, _>>()?;

        match kind {
            FfiNodeKind::Component(factory) => factory(id, children, context),

            FfiNodeKind::Root => Err(VkStatus::InvalidTreeNode),
        }
    }
}

pub(crate) struct FfiTree {
    root: FfiNode,
}

impl FfiTree {
    pub(crate) fn build(self, context: &mut FfiBuildContext) -> Result<Box<dyn View>, VkStatus> {
        let FfiNode { kind, children, .. } = self.root;

        if !matches!(kind, FfiNodeKind::Root) {
            return Err(VkStatus::InvalidTreeNode);
        }

        let children = children
            .into_iter()
            .map(|child| child.build(context).map(FfiBuiltView::into_stack_child))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Box::new(VStack::new().children(children)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FfiTreeBuilderError {
    NoOpenNode,
    UnclosedNodes,
    MultipleRoots,
    MissingRoot,
}

pub(crate) struct FfiTreeBuilder {
    stack: Vec<FfiNode>,
    roots: Vec<FfiNode>,
}

impl FfiTreeBuilder {
    pub(crate) fn new(root_node_id: u64) -> Self {
        Self {
            stack: vec![FfiNode::root(root_node_id)],

            roots: Vec::new(),
        }
    }

    pub(crate) fn begin(&mut self, node: FfiNode) {
        self.stack.push(node);
    }

    pub(crate) fn leaf(&mut self, node: FfiNode) {
        self.append(node);
    }

    pub(crate) fn end(&mut self) -> Result<(), FfiTreeBuilderError> {
        let node = self.stack.pop().ok_or(FfiTreeBuilderError::NoOpenNode)?;

        self.append(node);

        Ok(())
    }

    pub(crate) fn finish(self) -> Result<FfiTree, FfiTreeBuilderError> {
        if !self.stack.is_empty() {
            return Err(FfiTreeBuilderError::UnclosedNodes);
        }

        match self.roots.len() {
            0 => Err(FfiTreeBuilderError::MissingRoot),

            1 => Ok(FfiTree {
                root: self
                    .roots
                    .into_iter()
                    .next()
                    .expect("roots length was checked"),
            }),

            _ => Err(FfiTreeBuilderError::MultipleRoots),
        }
    }

    fn append(&mut self, node: FfiNode) {
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(node);
        } else {
            self.roots.push(node);
        }
    }
}

pub(crate) fn expect_no_children(children: Vec<FfiBuiltView>) -> Result<(), VkStatus> {
    if children.is_empty() {
        Ok(())
    } else {
        Err(VkStatus::InvalidChildCount)
    }
}

pub(crate) fn zero_or_one_view(children: Vec<FfiBuiltView>) -> Result<Box<dyn View>, VkStatus> {
    let mut children = children.into_iter();

    let child = children.next();

    if children.next().is_some() {
        return Err(VkStatus::InvalidChildCount);
    }

    Ok(match child {
        Some(child) => child.into_view(),

        None => Box::new(VStack::new()),
    })
}

pub(crate) fn zero_or_one_stack_child(children: Vec<FfiBuiltView>) -> Result<StackChild, VkStatus> {
    let mut children = children.into_iter();

    let child = children.next();

    if children.next().is_some() {
        return Err(VkStatus::InvalidChildCount);
    }

    Ok(match child {
        Some(child) => child.into_stack_child(),

        None => VStack::new().into_stack_child(),
    })
}

pub(crate) fn into_stack_children(children: Vec<FfiBuiltView>) -> Vec<StackChild> {
    children
        .into_iter()
        .map(FfiBuiltView::into_stack_child)
        .collect()
}
