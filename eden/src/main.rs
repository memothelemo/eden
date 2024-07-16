#[allow(clippy::unnecessary_wraps, clippy::unwrap_used)]
async fn bootstrap() -> eden_utils::Result<()> {
    println!("Hi!");
    Ok(())
}

#[allow(clippy::unwrap_used)]
fn main() {
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(bootstrap());

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
