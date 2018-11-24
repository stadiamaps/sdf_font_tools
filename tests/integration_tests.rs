extern crate futures;
extern crate tokio;

extern crate pbf_font_tools;

use futures::future::Future;

use std::path::Path;
use std::sync::mpsc;
use std::collections::HashMap;

#[test]
fn test_load_glyphs() {
    let font_path = Path::new("tests").join("glyphs");
    let (tx, rx) = mpsc::channel();

    let glyphs_fut = pbf_font_tools::load_glyphs(font_path.as_path(), "SeoulNamsan L", 0, 255)
        .then(move |result| {
            match result {
                Ok(glyphs) => {
                    // Make sure we have a font stack, and that it has the expected name
                    // and glyph count
                    let stack = &glyphs.get_stacks()[0];
                    let glyph_count = stack.get_glyphs().len();
                    let success = stack.get_name() == "SeoulNamsan L" && glyph_count == 170;

                    tx.send(success).expect("Failed to send message");
                    futures::future::ok(())
                }
                Err(_) => {
                    tx.send(false).expect("Failed to send message");
                    futures::future::err(())
                }
            }
        });
    tokio::run(glyphs_fut);

    let result = rx.recv().expect("Failed to receive message");
    assert!(result);
}

#[test]
fn test_get_font_stack() {
    let font_path = Path::new("tests").join("glyphs");
    let font_names = vec![
        String::from("SeoulNamsan L"),
        String::from("Open Sans Light"),
    ];
    let (tx, rx) = mpsc::channel();

    let namsan_font_future =
        pbf_font_tools::load_glyphs(font_path.as_path(), "SeoulNamsan L", 0, 255);

    let font_stack_fut = pbf_font_tools::get_font_stack(font_path.as_path(), font_names, 0, 255);

    let combinator = namsan_font_future.join(font_stack_fut);

    tokio::run(combinator.then(move |result| {
        match result {
            Ok((namsan_glyphs, combined_glyphs)) => {
                // Make sure we have a font stack, and that it has the expected name
                // and glyph count.
                let stack = &combined_glyphs.get_stacks()[0];
                let glyph_count = stack.get_glyphs().len();

                let mut success =
                    stack.get_name() == "SeoulNamsan L, Open Sans Light" && glyph_count == 228;

                let namsan_stack = &namsan_glyphs.get_stacks()[0];
                let namsan_mapping: HashMap<u32, Vec<u8>> = namsan_stack.get_glyphs().iter().map(|x| {
                    (x.get_id(), Vec::from(x.get_bitmap()))
                }).collect();

                // Make sure the Namsan font glyphs took precedence over the Open Sans ones.
                for glyph in stack.get_glyphs() {
                    if let Some(namsan_glyph) = namsan_mapping.get(&glyph.get_id()) {
                        if !namsan_glyph.eq(&Vec::from(glyph.get_bitmap())) {
                            success = false;
                        }
                    }
                }

                tx.send(success).expect("Failed to send message");
                futures::future::ok(())
            }
            Err(_) => {
                tx.send(false).expect("Failed to send message");
                futures::future::err(())
            }
        }
    }));

    let result = rx.recv().expect("Failed to receive message");
    assert!(result);
}
