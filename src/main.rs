use std::env;
use std::fs;
use std::path::Path;
use ic_file_uploader::{split_into_chunks, upload_chunk, create_error_string, MAX_CANISTER_HTTP_PAYLOAD_SIZE};


/// The maximum size of the HTTP payload for canister updates, set to 2 MiB.
pub const MAX_CANISTER_HTTP_PAYLOAD_SIZE: usize = 2 * 1000 * 1000; // 2 MiB

/// The main function for the ic-file-uploader crate.
///
/// This function processes command line arguments to determine the canister name, canister method name, file path, and optional arguments like offset, network type, and autoresume. It then reads the file, splits it into chunks, and uploads each chunk to the specified canister method.
///
/// # Returns
///
/// A `Result` indicating success (`Ok(())`) or an error message (`Err(String)`).
fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    // Ensure there are enough arguments
    if args.len() < 4 {
        return Err("Not enough arguments. Usage: <program> <canister_name> <canister_method_name> <file_path> [--offset <start_ind>] [--network <network_type>] [--autoresume <true/false>]".to_string());
    }

    // Default values for optional arguments
    let canister_name = &args[1];
    let canister_method_name = &args[2];
    let file_path = &args[3];
    let mut start_ind: usize = 0;
    let mut network_type: Option<&str> = None;
    let mut autoresume = false;  // likely need a canister call for this to get length

    // Parse arguments
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--offset" => {
                i += 1;
                start_ind = args[i].parse().map_err(|_| "Failed to parse start_ind as integer")?;
            }
            "--network" => {
                i += 1;
                network_type = Some(&args[i]);
            }
            "--autoresume" => {
                i += 1;
                autoresume = args[i].parse().map_err(|_| "Failed to parse autoresume as boolean")?;
            }
            _ => return Err(format!("Unknown argument: {}", args[i])),
        }
        i += 1;
    }

    let bytes_path = Path::new(file_path);
    println!("Uploading {}", file_path);

    let model_data = fs::read(&bytes_path).map_err(|e| e.to_string())?;
    let model_chunks = split_into_chunks(model_data, MAX_CANISTER_HTTP_PAYLOAD_SIZE, start_ind);

    for (index, model_chunk) in model_chunks.iter().enumerate() {
        // chunk number / index is a reference which is increasing
        // if we hit an error from upload chunk we should keep track of our current index
        // pause and retry
        if let Err(e) = upload_chunk(
            &format!("{canister_name} file"),
            canister_name,
            model_chunk,
            canister_method_name,
            index,
            model_chunks.len(),
            network_type,
        ) {
            eprintln!("Error uploading chunk {}: {}", index, e);
            return Err(format!("Upload interrupted at chunk {}: {}", index, e));
        }
    }

    Ok(())
}

