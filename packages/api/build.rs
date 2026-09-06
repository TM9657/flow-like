fn main() {
    // The embedded default remains tracked. OAuth and OpenID now consume the
    // same effective document selected at API startup.
    println!("cargo:rerun-if-changed=../../flow-like.config.json");
}
