//! This binary crate provides a CLI utility for batch converting a directory of fonts into
//! signed distance fields, encoded in a protocol buffer for renderers such as Mapbox GL. This
//! isn't really anything novel; it's just a frontend to
//! [pbf_font_tools](https://github.com/stadiamaps/pbf_font_tools) that behaves similar to
//! [node-fontnik](https://github.com/mapbox/node-fontnik), but is faster and (in our opinion)
//! a bit easier to use since it doesn't depend on node and all its headaches, or C++ libraries
//! that need to be built from scratch (this depends on `FreeType`, but that's widely available on
//! nearly any *nix-based system).
//!
//! Check out
//! [sdf_glyph_renderer](https://github.com/stadiamaps/sdf_glyph_renderer) for more technical
//! details on how this works.
//!
//! NOTE: This has requires you to have `FreeType` installed on your system. We recommend using
//! `FreeType` 2.10 or newer. Everything will still work against many older 2.x versions, but
//! the glyph generation improves over time so things will generally look better with newer
//! versions.
//!
//! ## Usage
//!
//! This tool will create `out_dir` if necessary, and will put each range (of 256 glyphs, for
//! compatibility with Mapbox fontstack convention) in a new subdirectory bearing the font name.
//! **Any existing glyphs will be overwritten in place.**
//!
//! ```
//! $ build_pbf_glyphs /path/to/font_dir /path/to/out_dir
//! ```

use std::collections::HashMap;
use std::ops::Deref;
use std::{
    fs::{create_dir_all, read_dir, File},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Instant,
};

use clap::{command, crate_authors, crate_description, crate_version, Arg};
use freetype::{Face, Library};
use protobuf::{CodedOutputStream, Message};
use spmc::{channel, Receiver};

static TOTAL_GLYPHS_RENDERED: AtomicUsize = AtomicUsize::new(0);

/// Combines glyphs for all fonts listed in `font_names` in `font_path` into a single stack
/// with name `stack_name`.
///
/// The font name list will be used as the order of precedence.
async fn combine_glyphs(font_path: PathBuf, font_names: &[&str], stack_name: String) {
    let out_dir = font_path.join(&stack_name);
    create_dir_all(&out_dir).expect("Unable to create output directory");

    let mut start = 0;
    let mut end = 255;
    let mut glyphs_combined = 0;

    while start < 65536 {
        let stack = pbf_font_tools::get_named_font_stack(
            &font_path,
            font_names,
            stack_name.clone(),
            start,
            end,
        )
        .await
        .expect("Unable to load font stack");

        // The above utility always returns a single stack
        glyphs_combined += stack.stacks[0].glyphs.len();

        let mut file = File::create(out_dir.join(format!("{}-{}.pbf", start, end)))
            .expect("Unable to create file");
        let mut cos = CodedOutputStream::new(&mut file);
        stack.write_to(&mut cos).expect("Unable to write");
        cos.flush().expect("Unable to flush");

        start += 256;
        end += 256;
    }

    println!(
        "Combined {} glyphs from [{}] into {}",
        glyphs_combined,
        font_names.join(", "),
        stack_name
    );
}

/// A worker function that converts a font to a set of SDF glyphs.
///
/// The glyphs are output as a set of files in a directory where each file contains
/// exactly 255 glyphs and is named like so: `<base_out_dir>/<font name>/<start>-<end>.pbf`
/// where the start and end numbers represent the unicade code point.
fn render_worker(
    base_out_dir: PathBuf,
    overwrite: bool,
    radius: usize,
    cutoff: f64,
    rx: Receiver<Option<(PathBuf, PathBuf)>>,
) {
    let lib = Library::init().expect("Unable to initialize FreeType");

    while let Ok(Some((path, stem))) = rx.recv() {
        let out_dir = base_out_dir.join(stem.to_str().expect("Unable to extract file stem"));
        create_dir_all(&out_dir).expect("Unable to create output directory");

        println!("Processing {}", path.to_str().unwrap());

        // Load the font once to save useless I/O
        let face = lib.new_face(&path, 0).expect("Unable to load font");
        let num_faces = face.num_faces() as usize;
        let faces: Vec<Face> = (0..num_faces)
            .map(|face_index| {
                lib.new_face(&path, face_index as isize)
                    .expect("Unable to load face")
            })
            .collect();

        let mut start = 0;
        let mut end = 255;
        let mut glyphs_rendered = 0;
        let mut glyphs_skipped = 0;
        let path_str = path
            .to_str()
            .expect("Unable to convert path to a valid UTF-8 string.");

        while start < 65536 {
            let glyph_path = out_dir.join(format!("{}-{}.pbf", start, end));
            if !overwrite && Path::exists(&glyph_path) {
                glyphs_skipped += 256;
            } else {
                let mut glyphs = pbf_font_tools::glyphs::Glyphs::new();

                for (face_index, face) in faces.iter().enumerate() {
                    if let Ok(stack) = pbf_font_tools::generate::glyph_range_for_face(
                        face, start, end, 24, radius, cutoff,
                    ) {
                        glyphs_rendered += stack.glyphs.len();
                        glyphs.stacks.push(stack);
                    } else {
                        println!(
                            "ERROR: Failed to render fontstack for face {} in {}",
                            face_index, path_str
                        )
                    }
                }

                let mut file = File::create(glyph_path).expect("Unable to create file");
                let mut cos = CodedOutputStream::new(&mut file);
                glyphs.write_to(&mut cos).expect("Unable to write");
                cos.flush().expect("Unable to flush");
            }

            start += 256;
            end += 256;
        }

        if glyphs_skipped > 0 {
            println!("Skipped up to {} glyphs in {}", glyphs_skipped, path_str);
        }
        if glyphs_skipped != 65536 {
            println!(
                "Found {} valid glyphs across {} face(s) in {}",
                glyphs_rendered, num_faces, path_str
            );
        }

        TOTAL_GLYPHS_RENDERED.fetch_add(glyphs_rendered, Ordering::Relaxed);
    }
}

fn main() {
    let matches = command!()
        .author(crate_authors!())
        .version(crate_version!())
        .before_help(crate_description!())
        .arg(Arg::new("FONT_DIR")
            .help("Sets the source directory to be scanned for fonts")
            .required(true)
            .index(1))
        .arg(Arg::new("OUT_DIR")
            .help("Sets the output directory in which the PBF glyphs will be placed (each font will be placed in a new subdirectory with appropriately named PBF files)")
            .required(true)
            .index(2))
        .arg(Arg::new("COMBINATION_SPEC")
            .help("Path to a file containing a set of glyph combination specifications. The file should contain a JSON dictionary having a format like so: {\"New Font Name\": [\"Font 1\", \"Font 2\"]}")
            .required(false)
            .short('c')
            .long("combinations")
            .takes_value(true))
        .arg(Arg::new("OVERWRITE")
            .help("Overwrite existing glyphs; by default, glyph generation will be skipped for any range with a matching file in the output directory. Note that the contents of the file are not inspected; only the name.")
            .required(false)
            .long("overwrite")
            .takes_value(false))
        .get_matches();

    let font_dir = Path::new(matches.get_one::<String>("FONT_DIR").unwrap());
    let out_dir = PathBuf::from(matches.get_one::<String>("OUT_DIR").unwrap());
    let overwrite = matches.is_present("OVERWRITE");

    let (mut tx, rx) = channel();
    let num_threads = num_cpus::get();
    println!("Starting {} worker threads...", num_threads);

    let join_handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let out_dir = out_dir.clone();
            let rx = rx.clone();
            thread::spawn(move || render_worker(out_dir, overwrite, 8, 0.25, rx))
        })
        .collect();

    let render_start = Instant::now();

    for dir_entry in read_dir(font_dir)
        .expect("Unable to open font directory")
        .flatten()
    {
        let path = dir_entry.path();

        if let (Some(stem), Some(extension)) = (path.file_stem(), path.extension()) {
            if path.is_file() && (["otf", "ttf", "ttc"].contains(&extension.to_str().unwrap())) {
                tx.send(Some((path.clone(), PathBuf::from(stem))))
                    .expect("Unable to push job to thread worker");
            }
        }
    }

    for _ in 0..num_threads {
        // Sentinel value to signal the end of the work pool for each thread
        tx.send(None)
            .expect("Unable to push completion job to thread worker");
    }

    for handle in join_handles {
        handle.join().unwrap();
    }

    let total_glyphs_rendered = TOTAL_GLYPHS_RENDERED.load(Ordering::Relaxed);
    let render_duration = render_start.elapsed();

    if total_glyphs_rendered > 0 {
        let duration_per_glyph = render_duration / total_glyphs_rendered as u32;

        println!(
            "Rendered {} glyph(s) in {:?} ({:?}/glyph)",
            total_glyphs_rendered, render_duration, duration_per_glyph
        );
    }

    if let Some(path) = matches.get_one::<String>("COMBINATION_SPEC") {
        // Async code, as necessary. Most of the rest of the code is actually truly blocking
        // since it's calling C libs or compute-heavy functions. Glyph combination however
        // happens to actually leverage async I/O, so we fire up a runtime here. It makes
        // the rest of the code simpler in this case to isolate the async code esp as it isn't
        // in the normal execution path.
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let data = tokio::fs::read(Path::new(path))
                    .await
                    .expect("Unable to read combination spec.");
                let combinations: HashMap<String, Vec<String>> =
                    serde_json::from_slice(&data).expect("Unable to parse combination spec.");
                for (name, fonts) in combinations.iter() {
                    let fonts: Vec<&str> = fonts.iter().map(|item| item.deref()).collect();
                    combine_glyphs(out_dir.clone(), &fonts, name.clone()).await
                }
            })
    }
}
