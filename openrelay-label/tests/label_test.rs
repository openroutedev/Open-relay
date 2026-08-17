use openrelay_label::format::PackageLabelData;
use openrelay_label::generator::LabelGenerator;
use openrelay_label::pdf::PackingSlipGenerator;

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
