use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::Path;

fn rgba_to_rgb565(rgba_data: &[u8]) -> Vec<u16> {
    let mut rgb565_data = Vec::new();

    for chunk in rgba_data.chunks(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        // Alpha channel ignored for RGB565

        // Convert to RGB565 format
        let r5 = (r >> 3) as u16;
        let g6 = (g >> 2) as u16;
        let b5 = (b >> 3) as u16;

        let rgb565 = (r5 << 11) | (g6 << 5) | b5;
        rgb565_data.push(rgb565);
    }

    rgb565_data
}

pub fn generate_all_frames() -> Result<(), Box<dyn std::error::Error>> {
    let gif_folder = Path::new("ui/assets/gif");

    for entry in fs::read_dir(gif_folder)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "gif") {
            let file_stem = path.file_stem().unwrap().to_str().unwrap().to_lowercase();
            let out_path = format!("src/{}_frames.rs", file_stem);
            println!("Processing {:?}", path);

            let input = File::open(&path)?;
            let mut decoder = gif::DecodeOptions::new();
            decoder.set_color_output(gif::ColorOutput::RGBA);
            let mut reader = decoder.read_info(input)?;

            let mut output = BufWriter::new(File::create(&out_path)?);

            writeln!(output, "// Auto-generated from {:?}", path)?;
            writeln!(output, "use crate::FrameData;\n")?;

            let mut frames = Vec::new();
            let mut frame_index = 0;

            while let Some(frame) = reader.read_next_frame()? {
                let delay_ms = frame.delay as u32 * 10;
                let rgb565_data = rgba_to_rgb565(&frame.buffer);

                writeln!(
                    output,
                    "const FRAME_{}_DATA: [u16; {}] = [",
                    frame_index,
                    rgb565_data.len()
                )?;

                for (i, pixel) in rgb565_data.iter().enumerate() {
                    if i % 16 == 0 {
                        write!(output, "    ")?;
                    }
                    write!(output, "0x{:04X}", pixel)?;
                    if i < rgb565_data.len() - 1 {
                        write!(output, ", ")?;
                    }
                    if (i + 1) % 16 == 0 {
                        writeln!(output)?;
                    }
                }
                if rgb565_data.len() % 16 != 0 {
                    writeln!(output)?;
                }
                writeln!(output, "];\n")?;

                frames.push((frame_index, delay_ms, frame.width, frame.height));
                frame_index += 1;
            }

            writeln!(
                output,
                "pub const {}_FRAMES: [FrameData; {}] = [",
                file_stem.to_uppercase(),
                frames.len()
            )?;
            for (index, delay, width, height) in &frames {
                writeln!(output, "    FrameData {{")?;
                writeln!(output, "        data: &FRAME_{}_DATA,", index)?;
                writeln!(output, "        delay_ms: {},", delay)?;
                writeln!(output, "        width: {},", width)?;
                writeln!(output, "        height: {},", height)?;
                writeln!(output, "    }},")?;
            }
            writeln!(output, "];\n")?;

            println!("Generated {} frames for {:?}", frames.len(), file_stem);
        }
    }

    Ok(())
}

fn main() {
    slint_build::compile("ui/home-window.slint").expect("Slint build failed");
    slint_build::compile("ui/roundprogress.slint").expect("Slint build failed");
    slint_build::compile("ui/splash-window.slint").expect("Slint build failed");
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
    embuild::espidf::sysenv::output();
    if let Err(e) = generate_all_frames() {
        println!("cargo:warning=Failed to generate frame data: {}", e);
    }



    slint_build::compile_with_config(
        "ui/app-window.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}
