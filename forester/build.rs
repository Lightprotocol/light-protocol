fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure().compile_protos(&["proto/photon.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/photon.proto");

    Ok(())
}
