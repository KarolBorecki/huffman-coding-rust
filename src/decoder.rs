mod huffman;

use crate::huffman::{
    CodeTable, FreqTable, build_code_table, build_huffman_tree, entropy_from_freq,
};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

fn read_frequencies_and_data_from_file(filepath: &str) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    eprintln!("[DEBUG] Reading file: {}", filepath);
    let content = fs::read(filepath)?;

    if content.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file is empty",
        ));
    }

    let freq_size = content[0] as usize + 2; // +1 for count byte, +1 to include the count byte itself
    eprintln!("[DEBUG] Total file size: {} bytes", content.len());
    eprintln!("[DEBUG] Header size (frequency table): {} bytes", freq_size);

    if freq_size > content.len() {
        eprintln!("[DEBUG] CRITICAL: Header size exceeds file content length!");
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "header size exceeds file content",
        ));
    }

    let freq_encoded = content[..freq_size].to_vec();
    let data_encoded = content[freq_size..].to_vec();

    eprintln!(
        "[DEBUG] Encoded data body size: {} bytes",
        data_encoded.len()
    );

    Ok((freq_encoded, data_encoded))
}

fn decode_frequencies(encoded: &[u8]) -> FreqTable {
    eprintln!("[DEBUG] Decoding frequency table...");
    let mut freq = HashMap::new();
    let count = (encoded[0] as usize) + 1; // +1 to account for the count byte itself

    eprintln!("[DEBUG] Frequency entries to process: {}", count);

    for i in 1..count+1 {
        if i >= encoded.len() {
            eprintln!(
                "[DEBUG] WARNING: Frequency table truncated unexpectedly at index {}",
                i
            );
            break;
        }
        let byte = encoded[i];
        freq.insert(byte, i as u64);
        eprintln!(
            "[DEBUG] Decoded frequency entry: byte {:#04x} ('{}') with frequency {}",
            byte, byte as char, i
        );
    }

    eprintln!(
        "[DEBUG] Reconstructed frequency map with {} unique symbols.",
        freq.len()
    );
    freq
}

fn decode_data(encoded: &[u8], code_table: &CodeTable) -> Vec<u8> {
    eprintln!("[DEBUG] Starting bitstream decoding...");
    let start_time = Instant::now();

    let mut bits = Vec::new();
    bits.reserve(encoded.len() * 8);

    for &byte in encoded {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    eprintln!(
        "[DEBUG] Expanded {} bytes into {} bits.",
        encoded.len(),
        bits.len()
    );

    let mut result = Vec::new();
    let mut current_code = String::new();

    let reverse_table: HashMap<String, u8> =
        code_table.iter().map(|(&b, c)| (c.clone(), b)).collect();

    eprintln!(
        "[DEBUG] Reverse lookup table created. Entries: {}",
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
    eprintln!("[DEBUG] Bitstream decoding finished in {:.2?}.", duration);
    eprintln!("[DEBUG] Final decoded data size: {} bytes.", result.len());

    result
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("⚠️ usage: {} <input_file> [output_file]", args[0]);
        eprintln!("  📂 <input_file>:  path to the encoded file.");
        eprintln!("  💾 <output_file>: path to write the decoded output.");
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let output_filepath = &args[2];

    eprintln!("--- [DEBUG] Start ---");

    let (encoded_freq, encoded_data) =
        read_frequencies_and_data_from_file(input_filepath).expect("failed to read encoded file");

    let decoded_freq = decode_frequencies(&encoded_freq);

    eprintln!("[DEBUG] Building Huffman Tree...");
    let decoded_tree = build_huffman_tree(&decoded_freq).expect("could not build huffman tree");

    let mut decoded_table = HashMap::new();
    build_code_table(&decoded_tree, String::new(), &mut decoded_table);
    eprintln!("[DEBUG] Code table built.");

    let decoded_data = decode_data(&encoded_data, &decoded_table);

    eprintln!("[DEBUG] Writing output to file: {}", output_filepath);
    let mut decoded_output_file =
        File::create(output_filepath).expect("cannot create decoded_output file");
    decoded_output_file
        .write_all(&decoded_data)
        .expect("could not write decoded data");

    eprintln!("[DEBUG] Write successful.");

    let input_size = fs::metadata(input_filepath).map(|m| m.len()).unwrap_or(0);
    let output_size = fs::metadata(output_filepath).map(|m| m.len()).unwrap_or(0);
    let file_entropy = entropy_from_freq(&decoded_freq);

    println!(
        "\r\n✅ decoding successful.\n\
         📂 input file:        {} ({} bytes)\n\
         💾 output file:       {} ({} bytes)\n\
         ℹ️ entropy:           {:.2} bits/symbol\n\
         🗜️ compression ratio: {:.2}%",
        input_filepath,
        input_size,
        output_filepath,
        output_size,
        file_entropy,
        100.0 * (1.0 - (output_size as f64) / (input_size as f64))
    );

    eprintln!("--- [DEBUG] End ---");
}
