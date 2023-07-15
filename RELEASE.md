# Release Guide

This workspace contains multiple crates. Updates can be performed individually in some cases, but note that there
are dependency orders to consider.

Always release `sdf_glyph_renderer` first.

## Release order

Create releases with tags according to the following convention.

* `sdf-vX.Y.Z` for `sdf_glyph_renderer` releases. Release this before other crates.
* `tools-vX.Y.Z` for `pbf_font_tools` releases. Release this next.
* `cli-vX.Y.Z` for `build_pbf_glyphs` releases. Release this last.
