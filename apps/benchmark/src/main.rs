use std::path::PathBuf;

fn main() {
    let base_path = PathBuf::from("packages/schema");
    flow_like::schema_gen::generate_schema(base_path.clone()).unwrap();
    println!("Schema generated in {:?}", base_path);
}
