extern crate pbf_font_tools;

use futures::future::join;
use std::collections::HashMap;
use std::path::Path;

#[tokio::test]
async fn test_load_glyphs() {
    let font_path = Path::new("tests").join("glyphs");
    let font_name = "SeoulNamsan L";
    let result = pbf_font_tools::load_glyphs(font_path.as_path(), font_name, 0, 255).await;

    match result {
        Ok(glyphs) => {
            let stack = &glyphs.get_stacks()[0];
            let glyph_count = stack.get_glyphs().len();
            assert_eq!(stack.get_name(), font_name);
            assert_eq!(glyph_count, 170);
        }
        Err(e) => panic!("Encountered error {:#?}.", e),
    }
}

#[tokio::test]
async fn test_get_font_stack() {
    let font_path = Path::new("tests").join("glyphs");
    let font_names = vec![
        String::from("SeoulNamsan L"),
        String::from("Open Sans Light"),
    ];

    let namsan_font_future =
        pbf_font_tools::load_glyphs(font_path.as_path(), "SeoulNamsan L", 0, 255);

    let font_stack_fut = pbf_font_tools::get_font_stack(font_path.as_path(), font_names, 0, 255);

    let result = join(namsan_font_future, font_stack_fut).await;

    match result {
        (Ok(namsan_glyphs), Ok(combined_glyphs)) => {
            // Make sure we have a font stack, and that it has the expected name
            // and glyph count.
            let stack = &combined_glyphs.get_stacks()[0];
            let glyph_count = stack.get_glyphs().len();

            assert_eq!(stack.get_name(), "SeoulNamsan L, Open Sans Light");
            assert_eq!(glyph_count, 228);

            let namsan_stack = &namsan_glyphs.get_stacks()[0];
            let namsan_mapping: HashMap<u32, Vec<u8>> = namsan_stack
                .get_glyphs()
                .iter()
                .map(|x| (x.get_id(), Vec::from(x.get_bitmap())))
                .collect();

            // Make sure the Namsan font glyphs took precedence over the Open Sans ones.
            for glyph in stack.get_glyphs() {
                if let Some(namsan_glyph) = namsan_mapping.get(&glyph.get_id()) {
                    if !namsan_glyph.eq(&Vec::from(glyph.get_bitmap())) {
                        panic!("Encountered glyph where Namsan was overwritten by Open Sans.");
                    }
                }
            }
        }
        _ => {
            panic!("One of the futures failed to complete successfully.");
        }
    }
}
