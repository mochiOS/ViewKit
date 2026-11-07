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

A shell panel that accepts text must request platform keyboard ownership as
well as focusing its local control. Use `EventContext::set_keyboard_capture(true)`
on opening and `set_keyboard_capture(false)` on every closing path. The runtime
applies the request to the originating window and sends `KeyboardCaptureDenied`
to its view tree if acquisition fails. Close the panel on denial or focus loss.
Ordinary Dock and menu interaction does not request capture.

On mochiOS, the compositor grants this only to an owned, visible Panel surface
with `FOCUS_CONTROL`, established when an authorized overlay client creates it.
Protected presentations, secure overlays and system modals block acquisition
and global shell shortcuts. Process modality is checked too. Release restores
the previous surface by its token; closing that surface falls back to a live
application. Preemption cancels the capture so a delayed release cannot steal
the new window's focus. Destroying the panel also releases its capture.

The internal compositor request is `OP_SET_KEYBOARD_CAPTURE` (13): exactly 16
bytes containing a little-endian u32 opcode, u64 surface token and u32 capture
flag (0 or 1). The reply uses the existing status format. The native backend
owns this protocol; applications use EventContext. No kernel API is added.
The Linux host backend accepts capture only while its shared preview window
already has host focus. Backends without this facility reject acquisition.
