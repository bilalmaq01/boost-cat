#[test]
fn sample_pdf_matches_independent_catalog() {
    let actual = boost_cat::import_pdf(std::path::Path::new("tests/fixtures/prepaid.pdf")).unwrap();
    let expected = boost_cat::read_csv(include_str!("../data/sample.csv")).unwrap();
    assert_eq!(actual.len(), 32);
    for (a, b) in actual.iter().zip(expected.iter()) {
        assert_eq!(a.model, b.model);
        assert_eq!(
            (
                a.upgrade,
                a.upgrade_plan,
                a.aal,
                a.aal_plan,
                a.port,
                a.port_plan,
                a.new,
                a.new_plan,
                a.months
            ),
            (
                b.upgrade,
                b.upgrade_plan,
                b.aal,
                b.aal_plan,
                b.port,
                b.port_plan,
                b.new,
                b.new_plan,
                b.months
            ),
            "{}",
            a.model
        );
    }
}
#[test]
fn unsupported_sheet_does_not_silently_import() {
    assert!(boost_cat::parse_pdf_text("Random prices $60 $20").is_err());
}
