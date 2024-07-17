//! This crate provides functionality for uploading files to Internet Computer canisters.
//!
//! It includes utilities for splitting files into chunks, converting data to blob strings,
//! and interfacing with the `dfx` command-line tool to upload data to canisters.

#![warn(missing_docs)]

use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;

/// The maximum size of the HTTP payload for canister updates, set to 2 MiB.
pub const MAX_CANISTER_HTTP_PAYLOAD_SIZE: usize = 2 * 1000 * 1000; // 2 MiB

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
pub fn split_into_chunks(data: Vec<u8>, chunk_size: usize, start_ind: usize) -> Vec<Vec<u8>> {
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
pub fn vec_u8_to_blob_string(data: &[u8]) -> String {
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
