use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use dicom_net::device::{ApplicationEntity, Connection, Device};
use dicom_net::scp::{CEchoService, CStoreService, FileCStoreSink};
use tracing::{Level, error};
use tracing_subscriber::EnvFilter;

/// DICOM C-STORE SCP
#[derive(Debug, Parser)]
#[command(version)]
struct App {
    /// Verbose mode
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
    /// Application Entity title
    #[arg(long = "ae-title", default_value = "STORESCP")]
    ae_title: String,
    /// Enforce max PDU length
    #[arg(short = 's', long = "strict")]
    strict: bool,
    /// Only accept native/uncompressed transfer syntaxes
    #[arg(long)]
    uncompressed_only: bool,
    /// Accept unknown SOP classes
    #[arg(long)]
    promiscuous: bool,
    /// Maximum PDU length
    #[arg(
        short = 'm',
        long = "max-pdu-length",
        default_value = "16378",
        value_parser(clap::value_parser!(u32).range(1018..))
    )]
    max_pdu_length: u32,
    /// Output directory for incoming objects
    #[arg(short = 'o', long = "output-dir", default_value = ".")]
    output_dir: PathBuf,
    /// Which port to listen on
    #[arg(short, long, default_value = "11111")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let app = App::parse();

    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(if app.verbose {
                Level::DEBUG
            } else {
                Level::INFO
            })
            .with_env_filter(
                EnvFilter::from_default_env().add_directive("dicom_net=info".parse().unwrap()),
            )
            .finish(),
    )
    .expect("Could not set up global logging subscriber");

    if let Err(e) = tokio::fs::create_dir_all(&app.output_dir).await {
        error!("Could not create output directory: {e}");
        std::process::exit(1);
    }

    let sink = Arc::new(FileCStoreSink::new(&app.output_dir));
    let cstore = if app.promiscuous {
        Arc::new(CStoreService::promiscuous(sink))
    } else {
        Arc::new(CStoreService::new(sink))
    };

    let bind_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, app.port));

    let conn = Connection::new()
        .port(bind_addr.port())
        .max_pdu_length(app.max_pdu_length)
        .strict(app.strict);

    let mut device = Device::new();
    let conn_index = device.add_connection(conn);

    let mut ae = ApplicationEntity::new(&app.ae_title)
        .acceptor(true)
        .promiscuous(app.promiscuous)
        .uncompressed_only(app.uncompressed_only)
        .add_connection(conn_index);
    ae.add_default_storage_capabilities();
    ae.register_service(Arc::new(CEchoService::new()));
    ae.register_cstore(cstore);
    device.add_application_entity(ae);

    let device = Arc::new(device);
    if let Err(e) = device.clone().bind_connections().await {
        error!("{e}");
        std::process::exit(1);
    }

    std::future::pending::<()>().await;
}
