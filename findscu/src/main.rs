use std::path::PathBuf;

use clap::Parser;
use dicom_core::header::Header;
use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::tags;
use dicom_net::device::{ApplicationEntity, Connection, TransferCapability};
use dicom_net::qr::STUDY_ROOT_FIND;
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use tracing::{Level, error, info};
use tracing_subscriber::EnvFilter;

/// DICOM Study Root C-FIND SCU
#[derive(Debug, Parser)]
#[command(version)]
struct App {
    /// Remote SCP address (example: `PACS@127.0.0.1:11112`)
    addr: String,
    /// Path to a pre-built C-FIND identifier dataset
    #[arg(long = "identifier")]
    identifier_file: Option<PathBuf>,
    /// Patient ID match key
    #[arg(long = "patient-id")]
    patient_id: Option<String>,
    /// Patient name match key (supports `*` wildcards)
    #[arg(long = "patient-name")]
    patient_name: Option<String>,
    /// Study Instance UID match key
    #[arg(long = "study-instance-uid")]
    study_instance_uid: Option<String>,
    /// Accession number match key
    #[arg(long = "accession-number")]
    accession_number: Option<String>,
    /// Calling Application Entity title
    #[arg(long = "calling-ae-title", default_value = "FINDSCU")]
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

    let mut scu = ApplicationEntity::new(&app.calling_ae_title).initiator(true);
    scu.add_scu_capability(TransferCapability::query_retrieve_find_scu(STUDY_ROOT_FIND));

    let matches = if let Some(path) = app.identifier_file {
        let bytes = std::fs::read(path)?;
        let mut assoc = scu.connect(&conn, &app.addr).await?;
        let matches = assoc.find(&bytes).await?;
        assoc.release().await?;
        matches
    } else if app.patient_name.is_some()
        || app.study_instance_uid.is_some()
        || app.accession_number.is_some()
    {
        let identifier = build_study_identifier(
            app.patient_id.as_deref(),
            app.patient_name.as_deref(),
            app.study_instance_uid.as_deref(),
            app.accession_number.as_deref(),
        )?;
        let mut assoc = scu.connect(&conn, &app.addr).await?;
        let matches = assoc.find(&identifier).await?;
        assoc.release().await?;
        matches
    } else {
        scu.find(&conn, &app.addr, app.patient_id.as_deref()).await?
    };

    info!(count = matches.len(), "C-FIND completed");
    for (index, dataset) in matches.iter().enumerate() {
        println!("--- match {index} ---");
        print_dataset(dataset)?;
    }
    Ok(())
}

fn build_study_identifier(
    patient_id: Option<&str>,
    patient_name: Option<&str>,
    study_instance_uid: Option<&str>,
    accession_number: Option<&str>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let obj = InMemDicomObject::from_element_iter([
        DataElement::new(
            tags::QUERY_RETRIEVE_LEVEL,
            VR::CS,
            dicom_value!(Str, "STUDY"),
        ),
        DataElement::new(
            tags::PATIENT_ID,
            VR::LO,
            dicom_value!(Str, patient_id.unwrap_or("")),
        ),
        DataElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            dicom_value!(Str, patient_name.unwrap_or("")),
        ),
        DataElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, study_instance_uid.unwrap_or("")),
        ),
        DataElement::new(
            tags::ACCESSION_NUMBER,
            VR::SH,
            dicom_value!(Str, accession_number.unwrap_or("")),
        ),
    ]);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    obj.write_dataset_with_ts(&mut data, &ts)?;
    Ok(data)
}

fn print_dataset(bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let obj = InMemDicomObject::read_dataset_with_ts(bytes, &ts)?;
    for elem in obj.iter() {
        let tag = elem.tag();
        let value = elem
            .to_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| format!("{:?}", elem.value()));
        println!("({:04X},{:04X}) {}", tag.group(), tag.element(), value);
    }
    Ok(())
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
