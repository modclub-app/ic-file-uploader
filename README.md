# ic-file-uploader

[![crates.io](https://img.shields.io/crates/v/ic-file-uploader.svg)](https://crates.io/crates/ic-file-uploader)
[![docs.rs](https://docs.rs/ic-file-uploader/badge.svg)](https://docs.rs/ic-file-uploader)

**ic-file-uploader** is a Rust crate designed to facilitate the efficient uploading of files to the Internet Computer. This crate focuses on breaking down large files into manageable chunks that fit within packet size limits and passing them to update calls which write these chunks to files.

## Features

- **Chunk-based Uploads**: Break large files into smaller chunks that can be easily managed and transferred.

## Use Cases

- **Large File Handling**: Efficiently manage and upload large singular files.

## License

All original work is licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT), at your option.

