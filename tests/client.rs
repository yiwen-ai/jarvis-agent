use tokio::fs::{read};


use reqwest::Identity;
use std::path::Path;

#[tokio::test(flavor = "current_thread")]
async fn run_client() -> anyhow::Result<()> {
    let cert = read(Path::new("tests/ca.crt")).await?;
    let cert = reqwest::Certificate::from_pem(&cert)?;

    let pem: Vec<u8> = read(Path::new("tests/client.pem")).await?;
    let identity = Identity::from_pem(&pem)?;

    let client = reqwest::Client::builder().use_rustls_tls();

    let client = client
        .tls_built_in_root_certs(false)
        .add_root_certificate(cert)
        .identity(identity)
        .https_only(true)
        .no_gzip()
        .build()?;

    let res = client
        .get("https://localhost:8443")
        .header("X-Forwarded-Host", "github.com")
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();

    println!("Status: {}", res.status());
    println!("Headers: {:?}", res.headers());

    let text = res.text().await.unwrap();
    if text.len() < 1024 {
        println!("Body: {} {:?}", text.len(), text);
    } else {
        println!("Body: {:?}", text.len());
    }

    Ok(())
}

async fn run_client_dev() -> anyhow::Result<()> {
    let cert = read(Path::new("debug/out/ca.crt")).await?;
    let cert = reqwest::Certificate::from_pem(&cert)?;

    let pem: Vec<u8> = read(Path::new("debug/out/client.pem")).await?;
    let identity = Identity::from_pem(&pem)?;

    let client = reqwest::Client::builder().use_rustls_tls();

    let client = client
        // .tls_built_in_root_certs(true)
        .add_root_certificate(cert)
        .identity(identity)
        .https_only(true)
        .build()?;

    let res = client
        .get("https://jarvis-agent.yiwen.ai:8443/yiwen-ai/jarvis-agent")
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
