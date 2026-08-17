pub mod format {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct PackageLabelData {
        pub commitment_hex: String,
        pub seal_serial: String,
        pub next_staging_point: String,
        pub deadline_timestamp: u64,
    }

    impl PackageLabelData {
        pub fn to_uri(&self) -> String {
            format!(
                "openrelay:pkg:v0.3/{}?seal={}&next={}&dl={}",
                self.commitment_hex, self.seal_serial, self.next_staging_point, self.deadline_timestamp
            )
        }
    }
}

pub mod generator {
    use super::format::PackageLabelData;
    use image::{DynamicImage, ImageBuffer, ImageOutputFormat, Luma};
    use qrcode::{EcLevel, QrCode};
    use std::io::Cursor;

    pub struct LabelGenerator;

    impl LabelGenerator {
        pub fn render_qr_png(data: &PackageLabelData) -> Result<Vec<u8>, String> {
            let uri = data.to_uri();
            let code = QrCode::with_error_correction_level(&uri, EcLevel::M)
                .map_err(|e| format!("{:?}", e))?;

            let image: ImageBuffer<Luma<u8>, Vec<u8>> = code.render::<Luma<u8>>()
                .quiet_zone(true)
                .min_dimensions(300, 300)
                .build();

            let dynamic_image = DynamicImage::ImageLuma8(image);
            let mut png_bytes: Vec<u8> = Vec::new();
            dynamic_image
                .write_to(&mut Cursor::new(&mut png_bytes), ImageOutputFormat::Png)
                .map_err(|e| format!("{:?}", e))?;

            Ok(png_bytes)
        }
    }
}

pub mod pdf {
    use super::format::PackageLabelData;
    use printpdf::*;
    use qrcode::{EcLevel, QrCode};

    pub struct PackingSlipGenerator;

    impl PackingSlipGenerator {
        pub fn generate_pdf(data: &PackageLabelData) -> Result<Vec<u8>, String> {
            let (doc, page1, layer1) = PdfDocument::new("OpenRelay Manifest", Mm(215.9), Mm(279.4), "Layer 1");
            let current_layer = doc.get_page(page1).get_layer(layer1);
            let font = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();

            current_layer.begin_text_section();
            current_layer.set_font(&font, 20.0);
            current_layer.set_text_cursor(Mm(20.0), Mm(255.0));
            current_layer.write_text("OPENRELAY PHYSICAL ROUTING SLIP", &font);
            current_layer.end_text_section();

            let uri = data.to_uri();
            let code = QrCode::with_error_correction_level(&uri, EcLevel::M)
                .map_err(|e| format!("{:?}", e))?;

            let module_size = 1.5;
            let base_x = 20.0;
            let base_y = 220.0;
            let width = code.width();
            let colors = code.to_colors();

            current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

            for y in 0..width {
                for x in 0..width {
                    if colors[y * width + x] == qrcode::Color::Dark {
                        let x0 = base_x + (x as f64) * module_size;
                        let y0 = base_y - (y as f64) * module_size;
                        let x1 = x0 + module_size;
                        let y1 = y0 - module_size;

                        let points = vec![
                            (Point::new(Mm(x0), Mm(y0)), false),
                            (Point::new(Mm(x1), Mm(y0)), false),
                            (Point::new(Mm(x1), Mm(y1)), false),
                            (Point::new(Mm(x0), Mm(y1)), false),
                        ];
                        let rect = Line {
                            points,
                            is_closed: true,
                            has_fill: true,
                            has_stroke: false,
                            is_clipping_path: false,
                        };
                        current_layer.add_shape(rect);
                    }
                }
            }

            let mut pdf_bytes = Vec::new();
            let mut writer = std::io::BufWriter::new(&mut pdf_bytes);
            doc.save(&mut writer).map_err(|e| format!("{:?}", e))?;
            drop(writer);

            Ok(pdf_bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format::PackageLabelData;
    use super::generator::LabelGenerator;
    use super::pdf::PackingSlipGenerator;

    #[test]
    fn test_label_and_pdf_generation() {
        let sample = PackageLabelData {
            commitment_hex: "a1b2c3d4".into(),
            seal_serial: "SEAL-001".into(),
            next_staging_point: "HUB-DEN".into(),
            deadline_timestamp: 1700000000,
        };

        assert!(LabelGenerator::render_qr_png(&sample).is_ok());
        assert!(PackingSlipGenerator::generate_pdf(&sample).is_ok());
    }
}
