use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::Path;
use qrcodegen::QrCode;
use qrcodegen::QrCodeEcc;
use sha256::{digest};

const DEVICE_ID: &str = "58db0095571ee686bdc5cfa3a7368eb9";
const SEERET_KEY: &str = "0bffd683ac83273d91c1d82d89f9d786";

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

fn rgb565_to_rgba8(rgb565_data: &[u16], width: u16, height: u16) -> Vec<u8> {
    let mut rgba_data = Vec::with_capacity((width as usize) * (height as usize) * 4);

    for &pixel in rgb565_data {
        let r = ((pixel >> 11) & 0x1F) as u8;
        let g = ((pixel >> 5) & 0x3F) as u8;
        let b = (pixel & 0x1F) as u8;

        let r8 = (r << 3) | (r >> 2);
        let g8 = (g << 2) | (g >> 4);
        let b8 = (b << 3) | (b >> 2);

        rgba_data.extend_from_slice(&[r8, g8, b8, 255]); // Alpha = 255
    }

    rgba_data
}

pub fn generate_rgba8_frames() -> Result<(), Box<dyn std::error::Error>> {
    let gif_folder = Path::new("ui/assets/gif");

    for entry in fs::read_dir(gif_folder)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "gif") {
            let file_stem = path.file_stem().unwrap().to_str().unwrap().to_lowercase();
            let out_path = format!("src/gif/{}_rgba8.rs", file_stem);
            println!("Processing RGBA8 {:?}", path);

            let input = File::open(&path)?;
            let mut decoder = gif::DecodeOptions::new();
            decoder.set_color_output(gif::ColorOutput::RGBA);
            let mut reader = decoder.read_info(input)?;

            let mut output = BufWriter::new(File::create(&out_path)?);

            writeln!(output, "// Auto-generated RGBA8 frame data from {:?}", path)?;
            writeln!(output, "use crate::RgbaFrameData;\n")?;

            let mut frames = Vec::new();
            let mut frame_index = 0;

            while let Some(frame) = reader.read_next_frame()? {
                let delay_ms = frame.delay as u32 * 10;
                let width = frame.width;
                let height = frame.height;

                // Convert original RGBA -> RGB565
                let rgb565_data = rgba_to_rgb565(&frame.buffer);
                // Then convert back to RGBA8 for uniformity
                let rgba8_data = rgb565_to_rgba8(&rgb565_data, width, height);

                writeln!(
                    output,
                    "const FRAME_{}_RGBA8: [u8; {}] = [",
                    frame_index,
                    rgba8_data.len()
                )?;

                for (i, byte) in rgba8_data.iter().enumerate() {
                    if i % 16 == 0 {
                        write!(output, "    ")?;
                    }
                    write!(output, "0x{:02X}", byte)?;
                    if i < rgba8_data.len() - 1 {
                        write!(output, ", ")?;
                    }
                    if (i + 1) % 16 == 0 {
                        writeln!(output)?;
                    }
                }
                if rgba8_data.len() % 16 != 0 {
                    writeln!(output)?;
                }

                writeln!(output, "];\n")?;

                frames.push((frame_index, delay_ms, width, height));
                frame_index += 1;
            }

            writeln!(
                output,
                "pub const {}_RGBA8_FRAMES: [RgbaFrameData; {}] = [",
                file_stem.to_uppercase(),
                frames.len()
            )?;
            for (index, delay, width, height) in &frames {
                writeln!(output, "    RgbaFrameData {{")?;
                writeln!(output, "        data: &FRAME_{}_RGBA8,", index)?;
                writeln!(output, "        delay_ms: {},", delay)?;
                writeln!(output, "        width: {},", width)?;
                writeln!(output, "        height: {},", height)?;
                writeln!(output, "    }},")?;
            }
            writeln!(output, "];\n")?;

            println!("Generated {} RGBA8 frames for {:?}", frames.len(), file_stem);
        }
    }
    Ok(())
}

fn to_svg_string(qr: &QrCode, border: i32) -> String {
	assert!(border >= 0, "Border must be non-negative");
	let mut result = String::new();
	result += "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
	result += "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n";
	let dimension = qr.size().checked_add(border.checked_mul(2).unwrap()).unwrap();
	result += &format!(
		"<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\">\n", dimension);
	result += "\t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>\n";
	result += "\t<path d=\"";
	for y in 0 .. qr.size() {
		for x in 0 .. qr.size() {
			if qr.get_module(x, y) {
				if x != 0 || y != 0 {
					result += " ";
				}
				result += &format!("M{},{}h1v1h-1z", x + border, y + border);
			}
		}
	}
	result += "\" fill=\"#000000\"/>\n";
	result += "</svg>\n";
	result
}

fn generate_qr_code() {
    let mac = "11:22:33:44:55:66";
    let input_sha = format!("{}-{}-{}", DEVICE_ID, SEERET_KEY, "1751897409");
    let val = digest(input_sha.clone());
    //let data_qr = format!("{}-{}-{}-{}", DEVICE_ID, "1751897409", mac, val);
    //let result = qrcode_generator::to_png_to_vec(&[DEVICE_ID, "1751897409", &mac, &val].concat(), QrCodeEcc::Low, 512).unwrap();
    let result = QrCode::encode_text(&[DEVICE_ID, "1751897409", mac, &val].concat(), QrCodeEcc::High).unwrap();
    let svg = to_svg_string(&result, 4);
    // Escape SVG string as raw Rust string
    // Write to src/qr.svg
    let output_path = Path::new("ui/assets/qr.svg");
    fs::write(&output_path, svg).expect("Failed to write src/qr.svg");
}
fn main() {
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
    embuild::espidf::sysenv::output();
    generate_qr_code();
    if let Err(e) = generate_rgba8_frames() {
        println!("cargo:warning=Failed to generate frame data: {}", e);
    }

    slint_build::compile_with_config(
        "ui/app-window.slint",
        slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer),
    )
    .unwrap();
}
