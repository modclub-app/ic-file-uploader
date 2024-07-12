use std::env;
use std::fs;
use std::process::Command;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

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

/// Splits the data into chunks.
///
/// # Arguments
///
/// * `data` - A vector of bytes representing the data to be split.
/// * `chunk_size` - The size of each chunk.
/// * `start_ind` - The starting index for chunking.
///
/// # Returns
///
/// A vector of byte vectors, each representing a chunk of the original data.
fn split_into_chunks(data: Vec<u8>, chunk_size: usize, start_ind: usize) -> Vec<Vec<u8>> {
    (start_ind..data.len())
        .step_by(chunk_size)
        .map(|start| {
            let end = usize::min(start + chunk_size, data.len());
            data[start..end].to_vec()
        })
        .collect()
}

/// Converts a vector of bytes to a blob string.
///
/// # Arguments
///
/// * `data` - A slice of bytes to be converted.
///
/// # Returns
///
/// A string representation of the blob data.
fn vec_u8_to_blob_string(data: &[u8]) -> String {
    let blob_content: String = data.iter().map(|&byte| format!("\\{:02X}", byte)).collect();
    format!("(blob \"{}\")", blob_content)
}

/// Uploads a chunk of data to the specified canister method.
///
/// # Arguments
///
/// * `name` - The name of the chunk being uploaded.
/// * `canister_name` - The name of the canister.
/// * `bytecode_chunk` - A reference to the vector of bytes representing the chunk.
/// * `canister_method_name` - The name of the canister method to call.
/// * `chunk_number` - The number of the current chunk.
/// * `chunk_total` - The total number of chunks.
/// * `network` - An optional network type.
///
/// # Returns
///
/// A `Result` indicating success (`Ok(())`) or an error message (`Err(String)`).
pub fn upload_chunk(name: &str,
    canister_name: &str,
    bytecode_chunk: &Vec<u8>,
    canister_method_name: &str,
    chunk_number: usize,
    chunk_total: usize,
    network: Option<&str>) -> Result<(), String> {

    let blob_string = vec_u8_to_blob_string(bytecode_chunk);

    let mut temp_file = NamedTempFile::new()
        .map_err(|_| create_error_string("Failed to create temporary file"))?;

    temp_file
        .as_file_mut()
        .write_all(blob_string.as_bytes())
        .map_err(|_| create_error_string("Failed to write data to temporary file"))?;

    let output = dfx(
        "canister",
        "call",
        &vec![
            canister_name,
            canister_method_name,
            "--argument-file",
            temp_file.path().to_str().ok_or(create_error_string(
                "temp_file path could not be converted to &str",
            ))?,
        ],
        network, // Pass the optional network argument
    )?;

    // 0-indexing to 1-indexing
    let chunk_number_display = chunk_number + 1;

    if output.status.success() {
        println!("Uploading {name} chunk {chunk_number_display}/{chunk_total}");
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        eprintln!("Failed to upload chunk {chunk_number_display}: {error_message}");
        return Err(create_error_string(&format!("Chunk {chunk_number_display} failed: {error_message}")));
    }

    Ok(())
}

/// Executes a dfx command with the specified arguments.
///
/// # Arguments
///
/// * `command` - The main dfx command to run.
/// * `subcommand` - The subcommand to execute.
/// * `args` - A vector of arguments for the command.
/// * `network` - An optional network type.
///
/// # Returns
///
/// A `Result` containing the output of the command or an error message.
pub fn dfx(command: &str, subcommand: &str, args: &Vec<&str>, network: Option<&str>) -> Result<std::process::Output, String> {
    let mut dfx_command = Command::new("dfx");
    dfx_command.arg(command);
    dfx_command.arg(subcommand);

    if let Some(net) = network {
        dfx_command.arg("--network");
        dfx_command.arg(net);
    }

    for arg in args {
        dfx_command.arg(arg);
    }

    dfx_command.output().map_err(|e| e.to_string())
}

/// Creates a formatted error string.
///
/// # Arguments
///
/// * `message` - The error message to format.
///
/// # Returns
///
/// A formatted error string.
pub fn create_error_string(message: &str) -> String {
    format!("Upload Error: {message}")
}
