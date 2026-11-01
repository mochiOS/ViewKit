# Application chrome alignment

Use `Toolbar` for application bars and `ContentArea::pane` for full-width
native detail content. Both use `theme.spacing.extra_large` as the horizontal
inset (24 logical pixels in the current theme). Pane content also starts at
that inset below its top edge. `ContentArea::new` retains the existing centered
page/form layout and width cap; `ContentArea::pane` has no implicit width cap.

Toolbar content is measured at its available width and centered vertically.
Its width remains the entire inset slot, so leading/trailing groups and spacers
continue to work. Views with zero intrinsic height retain their legacy full-slot
layout. Painting and event dispatch resolve the same content rectangle.
`NavigationLayout` uses the same centering rule for its toolbar region.

`ActionToolbar` owns its insets and already centers titles, icons and actions.
`Toolbar` and `NavigationLayout` recognize `View::owns_toolbar_insets`, so an
ActionToolbar can be placed inside either without doubling margins or height.
Box, Background and Surface preserve this layout property. Ordinary content
views should keep the default `false`; this is a container layout contract,
not a way to tune individual button padding.

A navigation split owns its separator. Pane backgrounds/content wrappers add
no separator or border of their own. Put one Divider between stacked regions
when the design needs a line; do not outline both adjoining surfaces. Divider
widths continue to use the existing theme token.

The changes do not introduce new spacing values, colors, fonts, control shapes
or animation parameters. Use the shared metrics and the same resolved frames
for painting, input and scroll viewports in custom layouts.

Native transparent surfaces can register extra pointer targets with
`DisplayList::add_input_region`. This is input metadata, not a transparent
paint layer: it changes neither pixels nor layout. Supply final presented,
surface-local logical rectangles each frame (at most 32); `clear` removes them.
The native backend converts them to physical pixels and only updates the
compositor when the region list changes. The compositor requires ownership and
commit rights for that surface; targets cannot escape its bounds, override
higher stacking layers, or intercept protected maximized window controls.
An empty list restores alpha-based panel input. Linux preview drawing transport
is unchanged; the host view must implement the matching click dispatch itself.
