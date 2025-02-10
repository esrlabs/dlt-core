use std::fs;

fn main() {
    let example_path = "examples/file_parser.rs";
    let readme_path = "README.md";

    let example_content = fs::read_to_string(example_path).expect("Failed to read example file");
    let readme_content = fs::read_to_string(readme_path).expect("Failed to read README file");

    let start_marker = "<!-- example start -->";
    let end_marker = "<!-- example end -->";

    // Find start and end markers
    if let (Some(start), Some(end)) = (
        readme_content.find(start_marker),
        readme_content.find(end_marker),
    ) {
        let before = &readme_content[..start + start_marker.len()];
        let after = &readme_content[end..];

        // Generate new README content
        let new_readme = format!("{}\n```rust\n{}\n```\n{}", before, example_content, after);

        // Write the updated README
        fs::write(readme_path, new_readme).expect("Failed to write updated README file");
    } else {
        panic!("README.md does not contain the required markers.");
    }

    // Ensure Cargo rebuilds when the example file changes
    println!("cargo:rerun-if-changed={}", example_path);
}
