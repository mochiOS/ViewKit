use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use super::{VK_EVENT_BUTTON_CLICKED, VkActionEvent, VkStatus};
use crate::components::{ScrollState, VStack};
use crate::layout::{IntoStackChild, StackAlignment, StackChild, StackGap};
use crate::state::{Binding, State};
use crate::view::View;

pub(crate) type SharedActionQueue = Rc<RefCell<VecDeque<VkActionEvent>>>;
pub(crate) type SharedStateStore = Rc<RefCell<FfiStateStore>>;

#[derive(Default)]
pub(crate) struct FfiStateStore {
    bools: HashMap<u64, State<bool>>,
    floats: HashMap<u64, State<f32>>,
    usizes: HashMap<u64, State<usize>>,
    strings: HashMap<u64, State<String>>,
    scrolls: HashMap<u64, ScrollState>,
}

impl FfiStateStore {
    fn bool_binding(&mut self, id: u64, initial: bool) -> Binding<bool> {
        self.bools
            .entry(id)
            .or_insert_with(|| State::new(initial))
            .binding()
    }

    fn float_binding(&mut self, id: u64, initial: f32) -> Binding<f32> {
        self.floats
            .entry(id)
            .or_insert_with(|| State::new(initial))
            .binding()
    }

    fn usize_binding(&mut self, id: u64, initial: usize) -> Binding<usize> {
        self.usizes
            .entry(id)
            .or_insert_with(|| State::new(initial))
            .binding()
    }

    fn string_binding(&mut self, id: u64, initial: String) -> Binding<String> {
        self.strings
            .entry(id)
            .or_insert_with(|| State::new(initial))
            .binding()
    }

    fn scroll_state(&mut self, id: u64) -> ScrollState {
        self.scrolls.entry(id).or_default().clone()
    }

    fn retain(&mut self, active: &HashSet<u64>) {
        self.bools.retain(|id, _| active.contains(id));

        self.floats.retain(|id, _| active.contains(id));

        self.usizes.retain(|id, _| active.contains(id));

        self.strings.retain(|id, _| active.contains(id));

        self.scrolls.retain(|id, _| active.contains(id));
    }
}

pub(crate) type FfiViewFactory =
    Box<dyn FnOnce(u64, Vec<FfiBuiltView>, &mut FfiBuildContext) -> Result<FfiBuiltView, VkStatus>>;

pub(crate) enum FfiBuiltView {
    View(Box<dyn View>),
    StackChild(StackChild),
    StackChildren(Vec<StackChild>),
}

impl FfiBuiltView {
    pub(crate) fn into_stack_children(self) -> Vec<StackChild> {
        match self {
            Self::View(view) => {
                vec![StackChild::new(view)]
            }

            Self::StackChild(child) => {
                vec![child]
            }

            Self::StackChildren(children) => children,
        }
    }

    pub(crate) fn into_stack_child(self) -> StackChild {
        let mut children = self.into_stack_children();

        if children.len() == 1 {
            return children.pop().expect("children length was checked");
        }

        VStack::new()
            .gap(StackGap::None)
            .alignment(StackAlignment::Start)
            .children(children)
            .into_stack_child()
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

            Self::StackChildren(children) => Box::new(
                VStack::new()
                    .gap(StackGap::None)
                    .alignment(StackAlignment::Start)
                    .children(children),
            ),
        }
    }
}

pub(crate) struct FfiBuildContext {
    component_instance_id: u64,

    actions: SharedActionQueue,
    states: SharedStateStore,

    active_state_ids: HashSet<u64>,
}

impl FfiBuildContext {
    pub(crate) fn new(
        component_instance_id: u64,
        actions: SharedActionQueue,
        states: SharedStateStore,
    ) -> Self {
        Self {
            component_instance_id,
            actions,
            states,
            active_state_ids: HashSet::new(),
        }
    }

    fn state_key(node_id: u64, state_id: u64) -> u64 {
        if state_id == 0 { node_id } else { state_id }
    }

    pub(crate) fn bool_binding(
        &mut self,
        node_id: u64,
        state_id: u64,
        initial: bool,
    ) -> Binding<bool> {
        let id = Self::state_key(node_id, state_id);

        self.active_state_ids.insert(id);

        self.states.borrow_mut().bool_binding(id, initial)
    }

    pub(crate) fn float_binding(
        &mut self,
        node_id: u64,
        state_id: u64,
        initial: f32,
    ) -> Binding<f32> {
        let id = Self::state_key(node_id, state_id);

        self.active_state_ids.insert(id);

        self.states.borrow_mut().float_binding(id, initial)
    }

    pub(crate) fn usize_binding(
        &mut self,
        node_id: u64,
        state_id: u64,
        initial: usize,
    ) -> Binding<usize> {
        let id = Self::state_key(node_id, state_id);

        self.active_state_ids.insert(id);

        self.states.borrow_mut().usize_binding(id, initial)
    }

    pub(crate) fn string_binding(
        &mut self,
        node_id: u64,
        state_id: u64,
        initial: String,
    ) -> Binding<String> {
        let id = Self::state_key(node_id, state_id);

        self.active_state_ids.insert(id);

        self.states.borrow_mut().string_binding(id, initial)
    }

    pub(crate) fn scroll_state(&mut self, node_id: u64, state_id: u64) -> ScrollState {
        let id = Self::state_key(node_id, state_id);

        self.active_state_ids.insert(id);

        self.states.borrow_mut().scroll_state(id)
    }

    pub(crate) fn retain_active_states(&self) {
        self.states.borrow_mut().retain(&self.active_state_ids);
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
        .flat_map(FfiBuiltView::into_stack_children)
        .collect()
}

pub(crate) fn exactly_two_stack_children(
    children: Vec<FfiBuiltView>,
) -> Result<(StackChild, StackChild), VkStatus> {
    let mut children = children.into_iter();

    let Some(first) = children.next() else {
        return Err(VkStatus::InvalidChildCount);
    };

    let Some(second) = children.next() else {
        return Err(VkStatus::InvalidChildCount);
    };

    if children.next().is_some() {
        return Err(VkStatus::InvalidChildCount);
    }

    Ok((first.into_stack_child(), second.into_stack_child()))
}
