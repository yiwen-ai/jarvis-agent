use axum::{middleware, routing, Router};
use futures_util::future::poll_fn;
use hyper::server::{
    accept::Accept,
    conn::{AddrIncoming, Http},
};
use reqwest::ClientBuilder;
use rustls_pemfile::{certs, read_one, Item};
use std::{fs::File, io::BufReader, net::SocketAddr, path::Path, pin::Pin, sync::Arc};
use structured_logger::{async_json::new_writer, Builder};
use tokio::{net::TcpListener, time::Duration};
use tokio_rustls::{
    rustls::{
        server::AllowAnyAuthenticatedClient, Certificate, PrivateKey, RootCertStore, ServerConfig,
    },
    TlsAcceptor,
};
use tower::{MakeService, ServiceBuilder};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::{predicate::SizeAbove, CompressionLayer},
};

mod agent;
mod conf;
mod context;
mod encoding;
mod erring;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let cfg = conf::Conf::new().unwrap_or_else(|err| panic!("config error: {}", err));

    Builder::with_level(cfg.log.level.as_str())
        .with_target_writer("*", new_writer(tokio::io::stdout()))
        .init();

    let server_cert = load_certs(&cfg.server.cert_file)?;
    let server_key = load_keys(&cfg.server.key_file)?;

    let client_root_cert = load_certs(&cfg.server.client_root_cert_file)?;
    let mut client_root_store = RootCertStore::empty();
    for cert in client_root_cert {
        client_root_store.add(&cert)?;
    }
    let client_auth = AllowAnyAuthenticatedClient::new(client_root_store);

    let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_client_cert_verifier(Arc::new(client_auth))
        .with_single_cert(server_cert, server_key)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    config.alpn_protocols = vec!["h2".into(), "http/1.1".into()];

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.server.port));
    let listener = TcpListener::bind(&addr).await?;
    let mut listener = AddrIncoming::from_listener(listener).unwrap();

    let acceptor = TlsAcceptor::from(Arc::new(config));
    let protocol = Arc::new(Http::new());

    let client = ClientBuilder::new()
        .http2_keep_alive_interval(Some(Duration::from_secs(25)))
        .http2_keep_alive_timeout(Duration::from_secs(5))
        .http2_keep_alive_while_idle(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(60))
        .gzip(false) // should be false, because we use compression middleware
        .build()
        .unwrap();

    let mds = ServiceBuilder::new()
        .layer(CatchPanicLayer::new())
        .layer(middleware::from_fn(context::middleware))
        .layer(CompressionLayer::new().compress_when(SizeAbove::new(encoding::MIN_ENCODING_SIZE)));

    let mut app = Router::new()
        .route("/", routing::any(agent::handler))
        .route("/*any", routing::any(agent::handler))
        .route_layer(mds)
        .with_state(client)
        .into_make_service_with_connect_info::<SocketAddr>();

    log::info!(
        "{}@{} start {} at {}",
        APP_NAME,
        APP_VERSION,
        cfg.env,
        &addr
    );
    loop {
        match poll_fn(|cx| Pin::new(&mut listener).poll_accept(cx)).await {
            None => {}
            Some(stream) => match stream {
                Ok(stream) => {
                    let acceptor = acceptor.clone();
                    let protocol = protocol.clone();
                    let svc = app.make_service(&stream);

                    let remote_addr = stream.remote_addr().to_string();
                    let local_addr = stream.local_addr().to_string();
                    let fut = async move {
                        let s = acceptor.accept(stream).await?;
                        let _ = protocol.serve_connection(s, svc.await.unwrap()).await;

                        Ok(()) as std::io::Result<()>
                    };

                    tokio::spawn(async move {
                        if let Err(err) = fut.await {
                            log::warn!(target: "server",
                                action = "spawn_service",
                                remote_addr = remote_addr,
                                local_addr = local_addr;
                                "{:?}", err,
                            );
                        }
                    });
                }
                Err(err) => {
                    log::warn!(target: "server",
                        action = "accept_stream";
                        "{:?}", err,
                    );
                }
            },
        }
    }
}

fn load_certs(path: &str) -> std::io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(Path::new(path))?))
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid cert at {}", path),
            )
        })
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_keys(path: &str) -> std::io::Result<PrivateKey> {
    match read_one(&mut BufReader::new(File::open(path)?))? {
        Some(Item::RSAKey(key)) => Ok(PrivateKey(key)),
        Some(Item::PKCS8Key(key)) => Ok(PrivateKey(key)),
        Some(Item::ECKey(key)) => Ok(PrivateKey(key)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid key at {}", path),
        )),
    }
}
