//! Integrated chrome using standard State-backed ViewKit components.
//! Run in mochiOS or through Binder's Linux application preview.
use viewkit::{
    components::{DragRegion, IconButtonInteractionState, TabsInteractionState, TitleBar},
    prelude::*,
};

struct Example {
    selection: State<usize>,
    tabs: State<Vec<usize>>,
    next: State<usize>,
    tab_interaction: TabsInteractionState,
    add_interaction: IconButtonInteractionState,
    menu: MenuButton,
}
impl App for Example {
    type Body = VStack;
    fn new() -> Self {
        Self {
            selection: State::new(1),
            tabs: State::new(vec![1, 2]),
            next: State::new(3),
            tab_interaction: TabsInteractionState::new(),
            add_interaction: IconButtonInteractionState::new(),
            menu: MenuButton::new("Window", SymbolName::More),
        }
    }
    fn window(&self) -> WindowOptions {
        let mut tabs =
            Tabs::with_interaction(self.selection.binding(), self.tab_interaction.clone())
                .compact_style();
        for id in self.tabs.get() {
            let items = self.tabs.clone();
            let selected = self.selection.clone();
            tabs = tabs.closable_item(id, format!("Document {id}"), move || {
                items.update(|tabs| tabs.retain(|item| *item != id));
                if selected.get() == id {
                    selected.set(items.with(|tabs| tabs.first().copied().unwrap_or(0)));
                }
            });
        }
        let items = self.tabs.clone();
        let next = self.next.clone();
        let selected = self.selection.clone();
        let add = IconButton::with_interaction(SymbolName::Plus, self.add_interaction.clone())
            .on_click(move || {
                let id = next.get();
                next.set(id + 1);
                items.update(|tabs| tabs.push(id));
                selected.set(id);
            });
        let items = self.tabs.clone();
        WindowOptions::new("Integrated Title Bar")
            .size(900.0, 600.0)
            .titlebar(
                TitleBar::new()
                    .height(44.0)
                    .leading(add)
                    .center(DragRegion::new(tabs))
                    .trailing(
                        self.menu
                            .clone()
                            .option("Reverse Tabs", move || items.update(|tabs| tabs.reverse())),
                    ),
            )
    }
    fn body(&self, _: &ViewContext) -> Self::Body {
        VStack::new()
            .child(Text::new(format!("Document {}", self.selection.get())))
            .child(Text::new(
                "Add, close or reorder tabs without recreating the window.",
            ))
            .child(DragRegion::new(Text::new(
                "Drag this text to move the window.",
            )))
    }
}
fn main() -> Result<(), viewkit::ViewKitError> {
    viewkit::run::<Example>()
}
