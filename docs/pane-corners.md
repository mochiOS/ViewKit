# Automatic pane corners

Pane corners belong to the layout/surface boundary, not to individual controls.
The runtime supplies the logical window bounds to both paint and event contexts.
`NavigationLayout` and `NavigationSplitView` wrap their detail region in a pane
surface. This works through stacks and other wrappers without changing child
frames, insets, scroll extents, or animation values.

For each pane, ViewKit compares its edges with the enclosing app surface. A
corner is rounded only where **both** adjoining edges are inside that surface:
for example, toolbar above + sidebar to the left produces a top-left corner;
a footer below the same pane also produces a bottom-left corner. An edge at
the window boundary stays square here: the window owns its outside shape.
An unsplit full-width content region is therefore not made into a floating card.

The radius is the existing `theme.radius.medium` setback. The existing continuous
corner geometry normalizes the four radii proportionally for small rectangles.
The same geometry drives clipping and input exclusion. The clip surrounds the
pane background and all its contents, including opaque backgrounds and scroll
indicators. There is no background-colored corner patch painted over children.

Custom app layouts can declare the relationship without choosing corner flags
or radii:

```rust,ignore
Surface::pane()
    .in_container(app_bounds)
    .content(detail_view)
    .paint(detail_bounds, context);
```

Use the same composed surface and rectangles for `handle_event`. Custom drawing
can use `paint_content` and `handle_content_event` with closures instead. Those
helpers preserve the child coordinate system. Excluded corner presses, wheel
and pointer-focus requests are rejected; move/release events in a cutout clear
child pointer tracking. Keyboard/menu and pointer cleanup broadcasts still flow.

`Surface::app()` starts a scoped enclosing surface when composing an independent
app region, including outside the window runtime. Nested scopes restore their
previous container after painting and event dispatch. Custom renderers that draw
backgrounds directly must opt into a pane surface; arbitrary rectangles are not
interpreted as semantic panes.
