use std::fs::File;
use std::io::Write;
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

fn generate_frame_data() -> Result<(), Box<dyn std::error::Error>> {
    use gif::{ColorOutput, DecodeOptions};

    // Read the GIF file
    let gif_path = "ui/assets/cat_1_resize.gif";

    let input = File::open(gif_path).unwrap();

    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);

    let mut reader = decoder.read_info(input).unwrap();

    let mut output = File::create("src/generated_frames.rs")?;

    // Write the header
    writeln!(output, "// Auto-generated frame data from GIF")?;
    writeln!(output, "use crate::FrameData;")?;
    writeln!(output, "")?;

    let mut frames = Vec::new();
    let mut frame_index = 0;

    // Process each frame
    while let Some(frame) = reader.read_next_frame()? {
        let delay_ms = frame.delay as u32 * 10; // Convert to milliseconds

        // Convert to RGB565 format to save memory (2 bytes per pixel instead of 4)
        let rgb565_data = rgba_to_rgb565(&frame.buffer);

        writeln!(
            output,
            "const FRAME_{}_DATA: [u16; {}] = [",
            frame_index,
            rgb565_data.len()
        )?;

        // Write data in chunks of 16 for readability
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
        writeln!(output, "];")?;
        writeln!(output)?;

        frames.push((frame_index, delay_ms, frame.width, frame.height));
        frame_index += 1;
    }

    // Generate the frame array
    writeln!(
        output,
        "pub const ANIMATION_FRAMES: [FrameData; {}] = [",
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
    writeln!(output, "];")?;

    println!("Generated {} frames", frames.len());
    Ok(())
}

fn main() {
    embuild::espidf::sysenv::output();
    // slint_build::compile("ui/app-window.slint").expect("Slint build failed");
    if let Err(e) = generate_frame_data() {
        println!("cargo:warning=Failed to generate frame data: {}", e);
    }

    // Rebuild if GIF file changes
    println!("cargo:rerun-if-changed=assets/animation.gif");

    slint_build::compile_with_config(
        "ui/app-window.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}
