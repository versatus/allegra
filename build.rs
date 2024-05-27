fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/allegra_rpc.proto")?;
    Ok(())
}
