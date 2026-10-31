fn main() {
    let imported = boost_cat::import_pdf(std::path::Path::new(
        &std::env::args().nth(1).expect("PDF path"),
    ))
    .unwrap();
    let seed = boost_cat::read_csv(include_str!("../data/sample.csv")).unwrap();
    assert_eq!(imported.len(), seed.len());
    for (a, b) in imported.iter().zip(&seed) {
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
    println!(
        "Verified all {} phones against independently extracted sample CSV",
        imported.len()
    );
}
