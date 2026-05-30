use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=frontend/dist");
    println!("cargo:rerun-if-changed=frontend/dist/index.html");

    if !Path::new("frontend/dist/index.html").is_file() {
        panic!(
            "frontend/dist is missing. Run `npm --prefix frontend ci && npm --prefix frontend run build` before `cargo build`."
        );
    }
}
