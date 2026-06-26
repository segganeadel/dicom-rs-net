use std::path::PathBuf;

use clap::Parser;
use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::tags;
use dicom_net::device::{ApplicationEntity, Connection, TransferCapability};
use dicom_net::qr::STUDY_ROOT_MOVE;
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use tracing::{Level, error, info};
use tracing_subscriber::EnvFilter;

/// DICOM Study Root C-MOVE SCU
#[derive(Debug, Parser)]
#[command(version)]
struct App {
    /// Remote QR SCP address (example: `PACS@127.0.0.1:11112`)
    addr: String,
    /// Move destination AE title
    #[arg(long = "destination", short = 'd')]
    move_destination: String,
    /// Path to a pre-built C-MOVE identifier dataset
    #[arg(long = "identifier")]
    identifier_file: Option<PathBuf>,
    /// SOP Instance UID for image-level move
    #[arg(long = "sop-instance-uid")]
    sop_instance_uid: Option<String>,
    /// Study Instance UID for study-level move
    #[arg(long = "study-instance-uid")]
    study_instance_uid: Option<String>,
    /// Calling Application Entity title
    #[arg(long = "calling-ae-title", default_value = "MOVESCU")]
    calling_ae_title: String,
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
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
            .with_env_filter(EnvFilter::from_default_env())
            .finish(),
    )
    .expect("Could not set up global logging subscriber");

    if let Err(e) = run(app).await {
        error!("{e}");
        std::process::exit(1);
    }
}

async fn run(app: App) -> Result<(), Box<dyn std::error::Error>> {
    let port = parse_port(&app.addr)?;
    let conn = Connection::new().port(port);

    let identifier = if let Some(path) = app.identifier_file {
        std::fs::read(path)?
    } else if let Some(sop_uid) = app.sop_instance_uid {
        build_image_identifier(&sop_uid)?
    } else if let Some(study_uid) = app.study_instance_uid {
        build_study_identifier(&study_uid)?
    } else {
        return Err("provide --identifier, --sop-instance-uid, or --study-instance-uid".into());
    };

    let mut scu = ApplicationEntity::new(&app.calling_ae_title).initiator(true);
    scu.add_scu_capability(TransferCapability::query_retrieve_move_scu(STUDY_ROOT_MOVE));

    let counts = scu
        .move_instances(&conn, &app.addr, &identifier, &app.move_destination)
        .await?;

    info!(
        completed = counts.completed,
        failed = counts.failed,
        "C-MOVE completed"
    );
    println!(
        "completed={} failed={} warning={}",
        counts.completed, counts.failed, counts.warning
    );
    Ok(())
}

fn build_image_identifier(sop_instance_uid: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let obj = InMemDicomObject::from_element_iter([
        DataElement::new(
            tags::QUERY_RETRIEVE_LEVEL,
            VR::CS,
            dicom_value!(Str, "IMAGE"),
        ),
        DataElement::new(
            tags::SOP_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, sop_instance_uid),
        ),
    ]);
    encode_identifier(&obj)
}

fn build_study_identifier(study_instance_uid: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let obj = InMemDicomObject::from_element_iter([
        DataElement::new(
            tags::QUERY_RETRIEVE_LEVEL,
            VR::CS,
            dicom_value!(Str, "STUDY"),
        ),
        DataElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, study_instance_uid),
        ),
    ]);
    encode_identifier(&obj)
}

fn encode_identifier(obj: &InMemDicomObject) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    obj.write_dataset_with_ts(&mut data, &ts)?;
    Ok(data)
}

fn parse_port(addr: &str) -> Result<u16, Box<dyn std::error::Error>> {
    let host_port = addr
        .split_once('@')
        .map(|(_, rest)| rest)
        .unwrap_or(addr);
    let port: u16 = host_port
        .rsplit_once(':')
        .ok_or("address must be AE@host:port or host:port")?
        .1
        .parse()?;
    Ok(port)
}
