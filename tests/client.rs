use tokio::fs::{read, File};
use tokio::io::AsyncReadExt;

use reqwest::Identity;
use std::path::Path;

#[tokio::test(flavor = "current_thread")]
async fn run_client() -> anyhow::Result<()> {
    let server_ca_file_loc = "tests/ca.crt";
    let mut buf = Vec::new();
    File::open(server_ca_file_loc)
        .await
        .unwrap()
        .read_to_end(&mut buf)
        .await
        .unwrap();
    let cert = reqwest::Certificate::from_pem(&buf)?;

    let pem: Vec<u8> = read(Path::new("tests/client.pem")).await?;
    let identity = Identity::from_pem(&pem)?;

    let client = reqwest::Client::builder().use_rustls_tls();

    let client = client
        .tls_built_in_root_certs(false)
        .add_root_certificate(cert)
        .identity(identity)
        .https_only(true)
        .build()?;

    let res = client
        .get("https://localhost:8443/")
        .header("X-Forwarded-Host", "github.com")
        .send()
        .await
        .unwrap();

    println!("Status: {}", res.status());
    println!("Headers: {:?}", res.headers());

    let text = res.text().await.unwrap();
    if text.len() < 1024 {
        println!("Body: {:?}", text);
    } else {
        println!("Body: {:?}", text.len());
    }

    Ok(())
}
