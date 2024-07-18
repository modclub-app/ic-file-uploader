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
}


/// The main function for the ic-file-uploader crate.
///
/// This function parses command line arguments, reads the specified file,
/// splits it into chunks, and uploads each chunk to the specified canister method.
fn main() -> Result<(), String> {
    let args = Args::parse();

    let bytes_path = Path::new(&args.file_path);
    println!("Uploading {}", args.file_path);

    let model_data = fs::read(&bytes_path).map_err(|e| e.to_string())?;
    let model_chunks = split_into_chunks(model_data, MAX_CANISTER_HTTP_PAYLOAD_SIZE, args.offset);

    for (index, model_chunk) in model_chunks.iter().enumerate() {
        if let Err(e) = upload_chunk(
            &format!("{} file", args.canister_name),
            &args.canister_name,
            model_chunk,
            &args.canister_method,
            index,
            model_chunks.len(),
            args.network.as_deref(),
        ) {
            eprintln!("Error uploading chunk {}: {}", index, e);
            return Err(format!("Upload interrupted at chunk {}: {}", index, e));
        }
    }

    // TODO: Implement autoresume functionality using the args.autoresume flag

    Ok(())
}

