use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "src/protobuf/app.proto",
            "src/protobuf/bit.proto",
            "src/protobuf/board.proto",
            "src/protobuf/comment.proto",
            "src/protobuf/node.proto",
            "src/protobuf/pin.proto",
            "src/protobuf/variable.proto",
        ],
        &["src/protobuf/"],
    )?;
    Ok(())
}
