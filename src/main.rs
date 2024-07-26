#![warn(missing_docs)]

//! This is a command-line tool for uploading files to Internet Computer canisters.
//!
//! It provides functionality to split files into chunks and upload them to specified canisters
//! using the Internet Computer protocol. The tool supports various options such as specifying
//! the canister name, method name, file path, and network type.

use std::fs;
use clap::Parser;
use std::path::Path;
use ic_file_uploader::{split_into_chunks, upload_chunk, MAX_CANISTER_HTTP_PAYLOAD_SIZE};
use futures::{stream, StreamExt};

/// Command line arguments for the ic-file-uploader
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the canister
    //#[arg(short, long)]
    canister_name: String,

    /// Name of the canister method
    //#[arg(short, long)]
    canister_method: String,

    /// Path to the file to be uploaded
    //#[arg(short, long)]
    file_path: String,

    /// Starting index for chunking (optional)
    #[arg(short, long, default_value = "0")]
    offset: usize,

    /// Network type (optional)
    #[arg(short, long)]
    network: Option<String>,

    /// Enable autoresume (optional, not yet implemented)
    #[arg(short, long, hide = true)]
    _autoresume: bool,

    /// Enable concurrent uploads
    #[arg(long, default_value_t = false)]
    concurrent: bool,

    /// Number of concurrent uploads (default: 5)
    #[arg(long, default_value = "5")]
    concurrent_uploads: usize,
}


/// The main function for the ic-file-uploader crate.
///
/// This function parses command line arguments, reads the specified file,
/// splits it into chunks, and uploads each chunk to the specified canister method.
#[tokio::main]
async fn main() -> Result<(), String> {
    let args = Args::parse();

    let bytes_path = Path::new(&args.file_path);
    println!("Uploading {}", args.file_path);

    let model_data = fs::read(&bytes_path).map_err(|e| e.to_string())?;
    let model_chunks = split_into_chunks(model_data, MAX_CANISTER_HTTP_PAYLOAD_SIZE, args.offset);

    // TODO: Implement autoresume functionality using the args.autoresume flag
    let model_chunks_len = model_chunks.len();
    if args.concurrent {
        /*
        let upload_futures = model_chunks.clone().into_iter().enumerate().map(|(index, chunk)| {
            upload_chunk(
                &args.canister_name,
                chunk,
                &args.canister_method,
                index,
                model_chunks_len,
                args.network.as_deref(),
                true,
            )
        });
        */
        let upload_futures = model_chunks.into_iter().enumerate().map(|(index, chunk)| {
            let canister_name = args.canister_name.clone();
            let canister_method = args.canister_method.clone();
            let network = args.network.clone();

            async move {
                upload_chunk(
                    &canister_name,
                    chunk,
                    &canister_method,
                    index,
                    model_chunks_len,
                    network.as_deref(),
                    true,
                ).await
            }
        });

        let results = stream::iter(upload_futures)
            .buffer_unordered(args.concurrent_uploads)
            .collect::<Vec<_>>()
            .await;

        for (index, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                eprintln!("Error uploading chunk {}: {}", index, e);
                return Err(format!("Upload interrupted at chunk {}: {}", index, e));
            }
        }

    } else {
       for (index, chunk) in model_chunks.into_iter().enumerate() {
            if let Err(e) = upload_chunk(
                &args.canister_name,
                chunk,
                &args.canister_method,
                index,
                model_chunks_len,
                args.network.as_deref(),
                false,
            ).await {
                eprintln!("Error uploading chunk {}: {}", index, e);
                return Err(format!("Upload interrupted at chunk {}: {}", index, e));
            }
        }
    }

    Ok(())
}


