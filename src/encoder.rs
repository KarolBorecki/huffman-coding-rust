mod huffman;

use std::env;
use std::fs::{self, File};
use std::io::Write;

use log::{debug, error, info};

use crate::huffman::{
    CodeTable, FreqTable, Symbol, build_code_table, build_huffman_tree, entropy_from_freq,
};

fn encode_frequencies(frequencies: &FreqTable, block_size: u8, original_len: u64) -> Vec<u8> {
    debug!("Generating frequency header with weights...");
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&original_len.to_be_bytes());
    bytes.push(block_size);

    let mut sorted_freq: Vec<(&Symbol, &u64)> = frequencies.iter().collect();
    sorted_freq.sort_by(|a, b| b.1.cmp(a.1));

    let unique_symbols = sorted_freq.len();
    debug!("Unique symbols to encode: {}", unique_symbols);

    bytes.extend_from_slice(&(unique_symbols as u32).to_be_bytes());

    for (symbol, freq) in sorted_freq {
        bytes.extend_from_slice(symbol);
        bytes.extend_from_slice(&freq.to_be_bytes());
    }

    debug!("Header generated. Total header size: {} bytes", bytes.len());
    bytes
}

fn encode_data(raw_data: &[u8], code_table: &CodeTable, order: usize) -> Vec<u8> {
    debug!("Starting context-aware data encoding...");
    let mut result = Vec::new();
    let mut current_byte = 0u8;
    let mut bit_count = 0;

    let mut context = vec![0u8; order];

    for &byte in raw_data {
        let mut symbol = context.clone();
        symbol.push(byte);

        if let Some(code) = code_table.get(&symbol) {
            for bit_char in code.chars() {
                let bit = if bit_char == '1' { 1 } else { 0 };

                current_byte = (current_byte << 1) | bit;
                bit_count += 1;

                if bit_count == 8 {
                    result.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }
        }

        if order > 0 {
            context.remove(0);
            context.push(byte);
        }
    }

    if bit_count > 0 {
        current_byte <<= 8 - bit_count;
        result.push(current_byte);
    }

    result
}
fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        error!("Usage: {} <input_file> [output_file] [--order=N]", args[0]);
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let mut output_filepath = "output.huff";
    let mut order = 0usize;

    for arg in &args[2..] {
        if arg.starts_with("--order=") {
            if let Ok(n) = arg.trim_start_matches("--order=").parse::<usize>() {
                order = n;
            }
        } else {
            output_filepath = arg;
        }
    }

    let block_size = order + 1;
    info!(
        "Encoding with Order: {} (Symbol size: {})",
        order, block_size
    );

    let raw_data = fs::read(input_filepath).expect("cannot read input file");
    let original_len = raw_data.len() as u64;

    let mut context = vec![0u8; order];
    let mut freq = FreqTable::new();

    for &byte in &raw_data {
        let mut sym = context.clone();
        sym.push(byte);
        *freq.entry(sym).or_insert(0) += 1;

        if order > 0 {
            context.remove(0);
            context.push(byte);
        }
    }

    let tree = build_huffman_tree(&freq).expect("could not build huffman tree");
    let mut table = CodeTable::new();
    build_code_table(&tree, String::new(), &mut table);

    let encoded_freq = encode_frequencies(&freq, block_size as u8, original_len);

    let encoded_data = encode_data(&raw_data, &table, order);

    let mut file = File::create(output_filepath).expect("cannot create output file");
    file.write_all(&encoded_freq).unwrap();
    file.write_all(&encoded_data).unwrap();

    let total_output_size = encoded_freq.len() + encoded_data.len();
    let file_entropy = entropy_from_freq(&freq);
    let compression_ratio = if original_len > 0 {
        100.0 * (1.0 - (total_output_size as f64) / (original_len as f64))
    } else {
        0.0
    };

    println!(
        "\r\n✅ Encoding successful.\n\
         📂  Input:       {} ({} bytes)\n\
         💾  Output:      {} ({} bytes)\n\
         ⚙️  Order:       {} (Symbol size: {})\n\
         ℹ️  Entropy:     {:.4} bits/symbol\n\
         🗜️  Ratio:       {:.4}%",
        input_filepath,
        original_len,
        output_filepath,
        total_output_size,
        order,
        block_size,
        file_entropy,
        compression_ratio
    );
}
