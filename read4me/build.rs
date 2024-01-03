fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["rpc/proto/yandex-cloud/tts.proto"],
            &["rpc/proto"],
        )?;
    Ok(())
}