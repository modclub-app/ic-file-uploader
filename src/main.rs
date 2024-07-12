

use std::env;
use std::fs;
use std::process::Command;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub const MAX_CANISTER_HTTP_PAYLOAD_SIZE: usize = 2 * 1000 * 1000; // 2 MiB


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
    let mut autoresume = false;  //likely need a canister call for this to get length

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



fn split_into_chunks(data: Vec<u8>, chunk_size: usize, start_ind: usize) -> Vec<Vec<u8>> {
    (start_ind..data.len())
        .step_by(chunk_size)
        .map(|start| {
            let end = usize::min(start + chunk_size, data.len());
            data[start..end].to_vec()
        })
        .collect()
}
/*
pub fn split_into_chunks(data: Vec<u8>, chunk_size: usize, start_ind: usize) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    let mut start = start_ind;
    let data_len = data.len();
    println!("Data Length {}", data_len);
    while start < data_len {
        let end = usize::min(start + chunk_size, data_len);
        chunks.push(data[start..end].to_vec());
        start = end;
    }
    chunks
}
*/


fn vec_u8_to_blob_string(data: &[u8]) -> String {
    let blob_content: String = data.iter().map(|&byte| format!("\\{:02X}", byte)).collect();
    format!("(blob \"{}\")", blob_content)
}



// this should return an index of last
pub fn upload_chunk(name: &str,
    canister_name: &str,
    bytecode_chunk: &Vec<u8>,
    canister_method_name: &str,
    chunk_number: usize,
    chunk_total: usize,
    network: Option<&str>) -> Result<(), String> {

    let blob_string = vec_u8_to_blob_string(bytecode_chunk);

    let mut temp_file =
        NamedTempFile::new()
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
            //&file_contents
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


pub fn create_error_string(message: &str) -> String {
    format!("Upload Error: {message}")
}
