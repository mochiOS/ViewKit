# Text and search fields

TextField shares the Small/Medium/Large control heights and horizontal padding
with standard buttons. Leading icons come from VK Symbols via leading_symbol;
search fields use SymbolName::Search. Placeholder, value, selection, caret and
pointer-to-character mapping resolve one geometry. There are no app-specific
pixel offsets for a search glyph or an input baseline.

The line box uses the same sanitized typography metrics and accessibility font
scale as Text. Its center matches the field and icon center; the default optical
offset is zero. Very large text is centered and clipped to the available field,
including its continuous corner shape. Horizontal padding and icon space shrink
safely for narrow fields. Long placeholders stay on a single clipped line.

Rest and disabled states retain the existing background/text tokens. Hover and
pointer editing use a neutral border. Keyboard traversal retains its focus cue;
pointer focus clears it, and ordinary typing after a pointer click does not
restore it. Validation errors keep their semantic border. Borderless fields
omit the external ring as well as background and border. Disabled glyphs use
the disabled text token, and disabling cancels transient pointer selection.

IME candidates and input-mode indicators are painted after the field interior
clip closes, so they can extend outside the field. Geometry changes do not alter
IME conversion, committed values, secure-value storage or clipboard policies.
