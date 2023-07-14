# SDF Font Tools

This is a collection of complementary crates for converting fonts into signed distance fields
and combining the results into MapLibre-compatible font stacks. Refer to the individual crate
READMEs for more details.

* [`build_pbf_glyphs`](build_pbf_glyphs) - CLI tool to crunch a directory fonts into PBF files you can host statically 
* [`pbf_font_tools`](pbf_font_tools) - Library exposing high level interfaces for generating glyphs from TTF/OTF fonts and combining glyphs from multiple fonts into fontstacks.
* [`sdf_glyph_renderer`](sdf_glyph_renderer) - Library crate for converting SDF glyphs from an arbitrary bitmap (alpha map).