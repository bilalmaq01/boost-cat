fn main() {
    println!(
        "{}",
        pdf_extract::extract_text(std::env::args().nth(1).unwrap()).unwrap()
    );
}
