extern crate pbf_font_tools;

use futures::future::join3;
use std::{collections::HashMap, path::Path};

#[tokio::test]
async fn test_load_glyphs() {
    let font_path = Path::new("tests").join("glyphs");
    let font_name = "SeoulNamsan L";
    let result = pbf_font_tools::load_glyphs(font_path.as_path(), font_name, 0, 255).await;

    match result {
        Ok(glyphs) => {
            let stack = &glyphs.stacks[0];
            let glyph_count = stack.glyphs.len();
            assert_eq!(stack.name, Some(String::from(font_name)));
            assert_eq!(glyph_count, 170);
        }
        Err(e) => panic!("Encountered error {:#?}.", e),
    }
}

#[tokio::test]
async fn test_get_font_stack() {
    let font_path = Path::new("tests").join("glyphs");
    let font_names = vec!["SeoulNamsan L", "Open Sans Light"];

    let namsan_font = pbf_font_tools::load_glyphs(font_path.as_path(), "SeoulNamsan L", 0, 255);

    let open_sans_font =
        pbf_font_tools::load_glyphs(font_path.as_path(), "Open Sans Light", 0, 255);

    let font_stack = pbf_font_tools::get_font_stack(font_path.as_path(), &font_names, 0, 255);

    match join3(namsan_font, open_sans_font, font_stack).await {
        (Ok(namsan_glyphs), Ok(open_sans_glyphs), Ok(combined_glyphs)) => {
            // Make sure we have a font stack, and that it has the expected name
            // and glyph count.
            let stack = &combined_glyphs.stacks[0];
            let glyph_count = stack.glyphs.len();

            assert_eq!(
                stack.name,
                Some(String::from("SeoulNamsan L, Open Sans Light"))
            );
            assert_eq!(glyph_count, 228);

            let namsan_stack = &namsan_glyphs.stacks[0];
            let namsan_mapping: HashMap<u32, Vec<u8>> = namsan_stack
                .glyphs
                .iter()
                .map(|x| {
                    (
                        x.id.expect("No id present"),
                        x.bitmap.clone().expect("No bitmap"),
                    )
                })
                .collect();

            let open_sans_stack = &open_sans_glyphs.stacks[0];
            let open_sans_mapping: HashMap<u32, Vec<u8>> = open_sans_stack
                .glyphs
                .iter()
                .map(|x| (x.id.unwrap(), x.bitmap.clone().unwrap()))
                .collect();

            let mut has_open_sans_glyph = false;

            // Make sure the Namsan font glyphs took precedence over the Open Sans ones.
            for glyph in &stack.glyphs {
                if let Some(namsan_glyph) = namsan_mapping.get(&glyph.id.unwrap()) {
                    if !namsan_glyph.eq(&glyph.bitmap.clone().unwrap()) {
                        panic!("Encountered glyph where Namsan was overwritten by Open Sans.");
                    }
                } else if open_sans_mapping.get(&glyph.id.unwrap()).is_some() {
                    has_open_sans_glyph = true;
                } else {
                    panic!("Uh, where did this glyph come from?");
                }
            }

            assert!(
                has_open_sans_glyph,
                "Should have at least one Open Sans glyph"
            );
        }
        _ => {
            panic!("One of the futures failed to complete successfully.");
        }
    }
}

#[cfg(feature = "freetype")]
#[tokio::test]
async fn test_glyph_generation() {
    let font_path = Path::new("tests").join("glyphs");
    let font_name = "Open Sans Light";
    let otf_path = font_path.join(font_name).join(format!("{}.ttf", font_name));
    let rendered_glyphs =
        pbf_font_tools::generate::glyph_range_for_font(&*otf_path, 0, 255, 24, 8, 0.25)
            .expect("Unable to render glyphs");
    let fixture_glyphs = pbf_font_tools::load_glyphs(font_path.as_path(), font_name, 0, 255)
        .await
        .expect("Unable to load fixtures");

    let rendered_stack = &rendered_glyphs.stacks[0];
    let fixture_stack = &fixture_glyphs.stacks[0];

    let rendered_glyph_count = rendered_stack.glyphs.len();
    let fixture_glyph_count = fixture_stack.glyphs.len();
    assert_eq!(rendered_glyph_count, fixture_glyph_count);

    rendered_stack
        .glyphs
        .iter()
        .zip(fixture_stack.glyphs.iter())
        .for_each(|(glyph, fixture)| assert_eq!(glyph, fixture))
}
