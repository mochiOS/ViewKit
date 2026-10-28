# List and sidebar row states

ListRow and SidebarItem use the same row appearance recipe and ListTokens.
Selection is persistent application state; hover, press and keyboard focus are
interaction state. Hover never replaces a selected row's background.

| State | Background | Primary text and symbol |
| --- | --- | --- |
| Rest | transparent | normal row foreground / leading foreground |
| Hover, active window | neutral surface-muted token | normal foreground |
| Selected, active window | existing accent-soft token | existing accent token |
| Selected, inactive window | neutral inactive-selection token | normal text token |
| Disabled | neutral selection when selected, otherwise transparent | disabled text token |

Inactive windows suppress hover and clear transient pointer tracking, preserving
selection. Runtime window activation comes from platform focus events and is
published through PaintContext::window_active; it is independent of the focused
text field or control. Standalone preview/custom drawing can supply the same
state with PaintContext::with_window_active. No per-row focus event is required
for a newly reconstructed row to receive the correct selection color.

Both controls use the existing row padding, medium icon-to-label spacing and
list-leading slot. The glyph is centered in that slot; reserve_symbol_space
keeps a symbol-less sidebar item aligned with its neighbors. Titles share the
label role and selection weight; supporting text uses the secondary row color.
Row surfaces do not inherit regular button shadows. Existing omochi behavior,
row heights, sidebar corner radius and connected-list corner rules are retained.

Virtualized collections should keep ButtonInteractionState per item and use
ListRow::with_interaction or SidebarItem::with_interaction. Keep selected state
in the application model and count logical neighbors for adjacent_rows. Fonts
retains its existing row gaps and verifies selection across activation changes.
