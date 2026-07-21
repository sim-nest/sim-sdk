# Diff A Modeled Halo Glance

This recipe reduces modeled Halo content to `scene/glance`, changes one glyph,
and emits only the changed Lua cell within the per-tick byte budget. A modeled
tap receives immediate local feedback through the shared `GlyphFlash` channel.
