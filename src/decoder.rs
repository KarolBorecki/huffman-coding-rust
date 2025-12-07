mod huffman;

use crate::huffman::{
    CodeTable, FreqTable, build_code_table, build_huffman_tree, entropy_from_freq,
};
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

fn read_frequencies_and_data_from_file(filepath: &str) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    info!("Reading encoded file: {}", filepath);
    let content = fs::read(filepath)?;

    if content.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file is empty",
        ));
    }

    let freq_size = content[0] as usize + 2;

    debug!("Total file size: {} bytes", content.len());
    debug!("Header size (frequency table): {} bytes", freq_size);

    if freq_size > content.len() {
        error!("CRITICAL: Header size exceeds file content length!");
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "header size exceeds file content",
        ));
    }

    let freq_encoded = content[..freq_size].to_vec();
    let data_encoded = content[freq_size..].to_vec();

    debug!("Encoded data body size: {} bytes", data_encoded.len());

    Ok((freq_encoded, data_encoded))
}

fn decode_frequencies(encoded: &[u8]) -> FreqTable {
    debug!("Decoding frequency table...");
    let mut freq = HashMap::new();
    let count = (encoded[0] as usize) + 1; // +1 because 0 represents 1 symbol

    debug!("Frequency entries to process: {}", count);

    for i in 1..count + 1 {
        if i >= encoded.len() {
            warn!("Frequency table truncated unexpectedly at index {}", i);
            break;
        }
        let byte = encoded[i];

        freq.insert(byte, i as u64);

        trace!(
            "Decoded frequency entry: byte {:#04x} ('{}') with assigned weight {}",
            byte, byte as char, i
        );
    }

    debug!(
        "Reconstructed frequency map with {} unique symbols.",
        freq.len()
    );
    freq
}

fn decode_data(encoded: &[u8], code_table: &CodeTable) -> Vec<u8> {
    debug!("Starting bitstream decoding...");
    let start_time = Instant::now();

    let mut bits = Vec::new();
    bits.reserve(encoded.len() * 8);

    for &byte in encoded {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }

    trace!("Expanded {} bytes into {} bits.", encoded.len(), bits.len());

    let mut result = Vec::new();
    let mut current_code = String::new();

    let reverse_table: HashMap<String, u8> =
        code_table.iter().map(|(&b, c)| (c.clone(), b)).collect();

    debug!(
        "Reverse lookup table created. Entries: {}",
        reverse_table.len()
    );

    for &bit in &bits {
        current_code.push(if bit == 1 { '1' } else { '0' });
        if let Some(&byte) = reverse_table.get(&current_code) {
            result.push(byte);
            current_code.clear();
        }
    }

    let duration = start_time.elapsed();
    debug!("Bitstream decoding finished in {:.2?}.", duration);
    debug!("Final decoded data size: {} bytes.", result.len());

    result
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        error!("Usage: {} <input_file> <output_file>", args[0]);
        eprintln!("  📂 <input_file>:  path to the encoded file.");
        eprintln!("  💾 <output_file>: path to write the decoded output.");
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let output_filepath = &args[2];

    info!("--- Start Decoding ---");

    let (encoded_freq, encoded_data) = match read_frequencies_and_data_from_file(input_filepath) {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to read encoded file: {}", e);
            std::process::exit(1);
        }
    };

    let decoded_freq = decode_frequencies(&encoded_freq);

    debug!("Building Huffman Tree...");
    let decoded_tree = match build_huffman_tree(&decoded_freq) {
        Some(t) => t,
        None => {
            error!("Could not build Huffman tree (frequency table might be empty).");
            std::process::exit(1);
        }
    };

    let mut decoded_table = HashMap::new();
    build_code_table(&decoded_tree, String::new(), &mut decoded_table);
    debug!("Code table built.");

    let decoded_data = decode_data(&encoded_data, &decoded_table);

    info!("Writing decoded output to file: {}", output_filepath);
    let mut decoded_output_file =
        File::create(output_filepath).expect("cannot create decoded_output file");

    if let Err(e) = decoded_output_file.write_all(&decoded_data) {
        error!("Could not write decoded data: {}", e);
        std::process::exit(1);
    }

    info!("Write successful.");

    let input_size = fs::metadata(input_filepath).map(|m| m.len()).unwrap_or(0);
    let output_size = fs::metadata(output_filepath).map(|m| m.len()).unwrap_or(0);
    let file_entropy = entropy_from_freq(&decoded_freq);

    let ratio = if input_size > 0 {
        100.0 * (1.0 - (input_size as f64) / (output_size as f64))
    } else {
        0.0
    };

    println!(
        "\r\n✅ decoding successful.\n\
         📂 input file:        {} ({} bytes)\n\
         💾 output file:       {} ({} bytes)\n\
         ℹ️ entropy:           {:.2} bits/symbol\n\
         🗜️ compression ratio: {:.2}% (relative to encoded input)",
        input_filepath, input_size, output_filepath, output_size, file_entropy, ratio
    );

    info!("--- End ---");
}
