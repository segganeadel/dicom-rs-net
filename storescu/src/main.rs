use std::path::PathBuf;

use clap::Parser;
use dicom_net::scu::{Client, StoreOptions};
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{Level, error};
use tracing_subscriber::EnvFilter;

/// DICOM C-STORE SCU
#[derive(Debug, Parser)]
#[command(version)]
struct App {
    /// Socket address of the Store SCP, optionally with AE title
    /// (example: "STORESCP@127.0.0.1:11111")
    addr: String,
    /// DICOM file(s) or directories to send
    #[arg(required = true)]
    files: Vec<PathBuf>,
    /// Verbose mode
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
    /// Calling Application Entity title
    #[arg(long = "calling-ae-title", default_value = "STORESCU")]
    calling_ae_title: String,
    /// Called Application Entity title (overrides AE in address if present)
    #[arg(long = "called-ae-title")]
    called_ae_title: Option<String>,
    /// Maximum PDU length
    #[arg(
        long = "max-pdu-length",
        default_value = "16378",
        value_parser(clap::value_parser!(u32).range(1018..))
    )]
    max_pdu_length: u32,
    /// Fail if not all DICOM files can be transferred
    #[arg(long = "fail-first")]
    fail_first: bool,
    /// Fail if transfer cannot be done without transcoding
    #[arg(long = "never-transcode")]
    never_transcode: bool,
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
        std::process::exit(-2);
    }
}

async fn run(app: App) -> Result<(), Box<dyn std::error::Error>> {
    let App {
        addr,
        files,
        verbose,
        calling_ae_title,
        called_ae_title,
        max_pdu_length,
        fail_first,
        never_transcode,
    } = app;

    let mut client = Client::new()
        .calling_ae(calling_ae_title)
        .remote(addr)
        .max_pdu_length(max_pdu_length)
        .never_transcode(never_transcode);

    if let Some(called) = called_ae_title {
        client = client.called_ae(called);
    }

    let store_options = StoreOptions {
        fail_first,
        never_transcode,
        verbose,
    };

    let file_count = files.len();
    let progress = if verbose {
        None
    } else {
        let pb = ProgressBar::new(file_count as u64);
        pb.set_style(
            ProgressStyle::with_template("{bar:40} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=> "),
        );
        Some(pb)
    };

    let sent = client.store_files(&files, &store_options).await?;

    if let Some(pb) = progress {
        pb.finish_with_message(format!("sent {sent} file(s)"));
    }

    if sent == 0 {
        return Err("no files were transferred".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::App;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        App::command().debug_assert();
    }
}
