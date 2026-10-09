fn main() {
    let acknowledgements = two_face::acknowledgement::listing();
    let mut markdown = format!(
        "# Embedded syntax acknowledgements\n\nSource: {}\n\n",
        two_face::acknowledgement::url()
    );
    for license in acknowledgements.for_syntaxes() {
        license.write_md(&mut markdown);
    }
    std::fs::write("licenses/SYNTAXES.md", markdown).expect("write syntax licenses");
}
