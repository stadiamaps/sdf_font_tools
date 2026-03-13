//! This binary crate provides a CLI utility for batch converting a directory of fonts into
//! signed distance fields, encoded in a protocol buffer for renderers such as Mapbox GL. This
//! isn't really anything novel; it's just a frontend to
//! [pbf_font_tools](https://github.com/stadiamaps/pbf_font_tools) that behaves similar to
//! [node-fontnik](https://github.com/mapbox/node-fontnik), but is faster and (in our opinion)
//! a bit easier to use since it doesn't depend on node and all its headaches, or C++ libraries
//! that need to be built from scratch. FreeType is bundled and compiled from source by default.
//!
//! Check out
//! [sdf_glyph_renderer](https://github.com/stadiamaps/sdf_glyph_renderer) for more technical
//! details on how this works.
//!
//! NOTE: The default `bitmap` backend uses FreeType (bundled from source by default).
//! The `bezier` backend uses a pure Rust font parser and does not require FreeType.
//! ## Usage
//!
//! This tool will create `out_dir` if necessary, and will put each range (of 256 glyphs, for
//! compatibility with Mapbox fontstack convention) in a new subdirectory bearing the font name.
//! **Any existing glyphs will be overwritten in place.**
//!
//! ```
//! $ build_pbf_glyphs /path/to/font_dir /path/to/out_dir
//! $ build_pbf_glyphs --backend bezier /path/to/font_dir /path/to/out_dir
//! ```

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;
use tokio::fs::{create_dir_all, File};

use clap::{Parser, ValueEnum};
use pbf_font_tools::freetype::{Face, Library};
use pbf_font_tools::ttf_parser;
use pbf_font_tools::{get_named_font_stack, glyph_range_for_face, Glyphs};
use prost::Message;
use spmc::{channel, Receiver};
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;

static TOTAL_GLYPHS_RENDERED: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Backend {
    /// Bitmap-based SDF using FreeType rasterization (default, fast).
    Bitmap,
    /// Vector-based SDF using bezier curves from font outlines (higher quality for complex scripts).
    Bezier,
}

#[derive(Parser, Debug)]
#[command(version, author, about)]
struct Args {
    /// Sets the source directory to be scanned for fonts.
    font_dir: PathBuf,
    /// Sets the output directory in which the PBF glyphs will be placed (each font will be placed in a new subdirectory with appropriately named PBF files).
    out_dir: PathBuf,
    /// Path to a file containing a set of glyph combination specifications. The file should contain a JSON dictionary having a format like so: {"New Font Name": ["Font 1", "Font 2"]}.
    #[arg(short, long = "combinations")]
    combinations_path: Option<String>,
    /// Overwrites existing glyphs. By default, glyph generation will be skipped for any range with a matching file in the output directory. Note that the contents of the file are not inspected; only the name.
    #[arg(long)]
    overwrite: bool,
    /// SDF generation backend to use.
    #[arg(long, value_enum, default_value_t = Backend::Bitmap)]
    backend: Backend,
}

/// Combines glyphs for all fonts listed in `font_names` in `font_path` into a single stack
/// with name `stack_name`.
///
/// The font name list will be used as the order of precedence.
async fn combine_glyphs(font_path: &Path, font_names: &[&str], stack_name: String) {
    let out_dir = font_path.join(&stack_name);
    create_dir_all(&out_dir)
        .await
        .expect("Unable to create output directory");

    let mut start = 0;
    let mut end = 255;
    let mut glyphs_combined = 0;

    while start < 65536 {
        let stack = get_named_font_stack(font_path, font_names, stack_name.clone(), start, end)
            .await
            .expect("Unable to load font stack");

        // The above utility always returns a single stack
        glyphs_combined += stack.stacks[0].glyphs.len();

        let encoded_bytes = spawn_blocking(move || stack.encode_to_vec())
            .await
            .expect("Unable to spawn an encoding task");
        let mut file = File::create(out_dir.join(format!("{start}-{end}.pbf")))
            .await
            .expect("Unable to create file");
        file.write_all(&encoded_bytes)
            .await
            .expect("Unable to write to file");

        start += 256;
        end += 256;
    }

    println!(
        "Combined {glyphs_combined} glyphs from [{}] into {stack_name}",
        font_names.join(", ")
    );
}

/// A worker function that converts a font to a set of SDF glyphs using FreeType (bitmap backend).
fn render_worker_bitmap(
    base_out_dir: &Path,
    overwrite: bool,
    radius: usize,
    cutoff: f64,
    rx: Receiver<Option<(PathBuf, PathBuf)>>,
) {
    let lib = Library::init().expect("Unable to initialize FreeType");

    while let Ok(Some((path, stem))) = rx.recv() {
        let out_dir = base_out_dir.join(stem.to_str().expect("Unable to extract file stem"));
        std::fs::create_dir_all(&out_dir).expect("Unable to create output directory");

        println!("Processing {}", path.display());

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
            let glyph_path = out_dir.join(format!("{start}-{end}.pbf"));
            if !overwrite && glyph_path.exists() {
                glyphs_skipped += 256;
            } else {
                let mut glyphs = Glyphs::default();

                for (face_index, face) in faces.iter().enumerate() {
                    if let Ok(stack) = glyph_range_for_face(face, start, end, 24, radius, cutoff) {
                        glyphs_rendered += stack.glyphs.len();
                        glyphs.stacks.push(stack);
                    } else {
                        println!(
                            "ERROR: Failed to render fontstack for face {face_index} in {path_str}",
                        );
                    }
                }

                let encoded_bytes = glyphs.encode_to_vec();
                let mut file = std::fs::File::create(glyph_path).expect("Unable to create file");
                file.write_all(&encoded_bytes)
                    .expect("Unable to write to file");
            }

            start += 256;
            end += 256;
        }

        if glyphs_skipped > 0 {
            println!("Skipped up to {glyphs_skipped} glyphs in {path_str}");
        }
        if glyphs_skipped != 65536 {
            println!(
                "Found {glyphs_rendered} valid glyphs across {num_faces} face(s) in {path_str}"
            );
        }

        TOTAL_GLYPHS_RENDERED.fetch_add(glyphs_rendered, Ordering::Relaxed);
    }
}

/// A worker function that converts a font to a set of SDF glyphs using ttf-parser (bezier backend).
fn render_worker_bezier(
    base_out_dir: &Path,
    overwrite: bool,
    radius: usize,
    cutoff: f64,
    rx: Receiver<Option<(PathBuf, PathBuf)>>,
) {
    use pbf_font_tools::glyph_range_for_face_ttf;

    while let Ok(Some((path, stem))) = rx.recv() {
        let out_dir = base_out_dir.join(stem.to_str().expect("Unable to extract file stem"));
        std::fs::create_dir_all(&out_dir).expect("Unable to create output directory");

        println!("Processing {} (bezier)", path.display());

        let font_data = std::fs::read(&path).expect("Unable to read font file");
        let num_faces = ttf_parser::fonts_in_collection(&font_data).unwrap_or(1);

        let mut start = 0u32;
        let mut end = 255u32;
        let mut glyphs_rendered = 0;
        let mut glyphs_skipped = 0;
        let path_str = path
            .to_str()
            .expect("Unable to convert path to a valid UTF-8 string.");

        while start < 65536 {
            let glyph_path = out_dir.join(format!("{start}-{end}.pbf"));
            if !overwrite && glyph_path.exists() {
                glyphs_skipped += 256;
            } else {
                let mut glyphs = Glyphs::default();

                for face_index in 0..num_faces {
                    match ttf_parser::Face::parse(&font_data, face_index) {
                        Ok(face) => {
                            if let Ok(stack) = glyph_range_for_face_ttf(
                                &face, start, end, 24.0, radius, cutoff,
                            ) {
                                glyphs_rendered += stack.glyphs.len();
                                glyphs.stacks.push(stack);
                            } else {
                                println!(
                                    "ERROR: Failed to render fontstack for face {face_index} in {path_str}",
                                );
                            }
                        }
                        Err(e) => {
                            println!(
                                "ERROR: Failed to parse face {face_index} in {path_str}: {e}",
                            );
                        }
                    }
                }

                let encoded_bytes = glyphs.encode_to_vec();
                let mut file = std::fs::File::create(glyph_path).expect("Unable to create file");
                file.write_all(&encoded_bytes)
                    .expect("Unable to write to file");
            }

            start += 256;
            end += 256;
        }

        if glyphs_skipped > 0 {
            println!("Skipped up to {glyphs_skipped} glyphs in {path_str}");
        }
        if glyphs_skipped != 65536 {
            println!(
                "Found {glyphs_rendered} valid glyphs across {num_faces} face(s) in {path_str}"
            );
        }

        TOTAL_GLYPHS_RENDERED.fetch_add(glyphs_rendered, Ordering::Relaxed);
    }
}

fn main() {
    let args = Args::parse();

    let font_dir = &args.font_dir;
    let out_dir = &args.out_dir;
    let backend = args.backend;

    let (mut tx, rx) = channel();
    let num_threads = thread::available_parallelism().unwrap().get();
    println!(
        "Starting {num_threads} worker threads (backend: {})...",
        match backend {
            Backend::Bitmap => "bitmap",
            Backend::Bezier => "bezier",
        }
    );

    let join_handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let out_dir = out_dir.clone();
            let rx = rx.clone();
            match backend {
                Backend::Bitmap => {
                    thread::spawn(move || {
                        render_worker_bitmap(&out_dir, args.overwrite, 8, 0.25, rx)
                    })
                }
                Backend::Bezier => {
                    thread::spawn(move || {
                        render_worker_bezier(&out_dir, args.overwrite, 8, 0.25, rx)
                    })
                }
            }
        })
        .collect();

    let render_start = Instant::now();

    for dir_entry in font_dir
        .read_dir()
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
            "Rendered {total_glyphs_rendered} glyph(s) in {render_duration:?} ({duration_per_glyph:?}/glyph)"
        );
    }

    if let Some(path) = args.combinations_path {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let data = tokio::fs::read(path)
                    .await
                    .expect("Unable to read combination spec.");
                let combinations: HashMap<String, Vec<String>> =
                    serde_json::from_slice(&data).expect("Unable to parse combination spec.");
                for (name, fonts) in combinations {
                    let fonts: Vec<&str> = fonts.iter().map(|item| item.as_str()).collect();
                    combine_glyphs(out_dir, &fonts, name.clone()).await;
                }
            });
    }
}
