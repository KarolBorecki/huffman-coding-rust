mod huffman;

use crate::huffman::{build_code_table, build_huffman_tree, CodeTable, entropy_from_freq, FreqTable};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;

fn read_frequencies_and_data_from_file(filepath: &str) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    let content = fs::read(filepath)?;
    if content.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "file is empty",
        ));
    }
    let freq_size = content[0] as usize + 1;
    let freq_encoded = content[..freq_size].to_vec();
    let data_encoded = content[freq_size..].to_vec();
    Ok((freq_encoded, data_encoded))
}

fn decode_frequencies(encoded: &[u8]) -> FreqTable {
    let mut freq = HashMap::new();
    let count = encoded[0] as usize;
    for i in 0..count {
        let byte = encoded[i + 1];
        freq.insert(byte, (i + 1) as u64);
    }
    freq
}

fn decode_data(encoded: &[u8], code_table: &CodeTable) -> Vec<u8> {
    let mut bits = Vec::new();
    for &byte in encoded {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }

    let mut result = Vec::new();
    let mut current_code = String::new();
    let reverse_table: HashMap<String, u8> =
        code_table.iter().map(|(&b, c)| (c.clone(), b)).collect();

    for &bit in &bits {
        current_code.push(if bit == 1 { '1' } else { '0' });
        if let Some(&byte) = reverse_table.get(&current_code) {
            result.push(byte);
            current_code.clear();
        }
    }
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

    let (encoded_freq, encoded_data) =
        read_frequencies_and_data_from_file(input_filepath).expect("failed to read encoded file");

    let decoded_freq = decode_frequencies(&encoded_freq);

    let decoded_tree = build_huffman_tree(&decoded_freq).unwrap();

    let mut decoded_table = HashMap::new();
    build_code_table(&decoded_tree, String::new(), &mut decoded_table);

    let decoded_data = decode_data(&encoded_data, &decoded_table);
    let mut decoded_output_file =
        File::create(output_filepath).expect("cannot create decoded_output file");
    decoded_output_file
        .write_all(&decoded_data)
        .expect("could not write decoded data");
        let input_size = fs::metadata(input_filepath)
        .map(|m| m.len())
        .unwrap_or(0);

    let output_size = fs::metadata(output_filepath)
        .map(|m| m.len())
        .unwrap_or(0);

    let file_entropy = entropy_from_freq(&decoded_freq);

    println!(
        "✅ decoding successful.\n\
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
}
