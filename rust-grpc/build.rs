fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::warning=build.rs is running");
    println!("cargo::rerun-if-changed=proto/inventory.proto");

    tonic_prost_build::compile_protos("proto/inventory.proto")?;

    Ok(())
}
