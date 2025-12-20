mod huffman;

use std::collections::BinaryHeap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use log::{debug, error, info, warn};

use crate::huffman::{
    build_code_table, build_huffman_tree, entropy_from_freq, CodeTable, FreqTable, Node, Symbol,
};

fn encode_frequencies(frequencies: &FreqTable, block_size: u8, original_len: u64) -> Vec<u8> {
    debug!("Generating frequency header with weights...");
    let mut bytes = Vec::new();

    // 1. Oryginalna długość (8B)
    bytes.extend_from_slice(&original_len.to_be_bytes());
    // 2. Rozmiar bloku (1B)
    bytes.push(block_size);

    // Sortujemy dla porządku w pliku (nie jest to stricte wymagane dla logiki, ale pomaga w debugowaniu)
    let mut sorted_freq: Vec<(&Symbol, &u64)> = frequencies.iter().collect();
    // Sortujemy malejąco po wadze
    sorted_freq.sort_by(|a, b| b.1.cmp(a.1));

    let unique_symbols = sorted_freq.len();
    debug!("Unique symbols to encode: {}", unique_symbols);

    // 3. Liczba wpisów w tabeli (4B)
    bytes.extend_from_slice(&(unique_symbols as u32).to_be_bytes());

    // 4. Pary: [Symbol (block_size bytes)] + [Freq (8 bytes)]
    for (symbol, freq) in sorted_freq {
        bytes.extend_from_slice(symbol);
        bytes.extend_from_slice(&freq.to_be_bytes());
    }

    debug!("Header generated. Total header size: {} bytes", bytes.len());
    bytes
}

fn encode_data(data_blocks: &[Vec<u8>], code_table: &CodeTable) -> Vec<u8> {
    debug!("Starting data encoding...");
    let mut bits = Vec::with_capacity(data_blocks.len() * 8);

    for block in data_blocks {
        if let Some(code) = code_table.get(block) {
            for c in code.chars() {
                bits.push(if c == '1' { 1 } else { 0 });
            }
        } else {
            error!("CRITICAL: Symbol {:?} found in data but not in code table!", block);
        }
    }

    // Padding
    while bits.len() % 8 != 0 {
        bits.push(0);
    }

    let mut bytes = Vec::with_capacity(bits.len() / 8);
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for &bit in chunk {
            byte = (byte << 1) | bit;
        }
        bytes.push(byte);
    }
    bytes
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
                order = if n <= 2 { n } else { 2 };
            }
        } else {
            output_filepath = arg;
        }
    }

    let block_size = order + 1;
    info!("Encoding with Order: {} (BlockSize: {})", order, block_size);

    let raw_data = fs::read(input_filepath).expect("cannot read input file");
    let original_len = raw_data.len() as u64;

    // Blocking logic
    let mut chunks: Vec<Symbol> = raw_data
        .chunks(block_size)
        .map(|chunk| {
            let mut c = chunk.to_vec();
            while c.len() < block_size { c.push(0); }
            c
        })
        .collect();

    let mut freq = FreqTable::new();
    for block in &chunks {
        *freq.entry(block.clone()).or_insert(0) += 1;
    }

    // Budowa drzewa (teraz z prawdziwymi wagami!)
    let tree = build_huffman_tree(&freq).expect("could not build huffman tree");
    
    let mut table = CodeTable::new();
    build_code_table(&tree, String::new(), &mut table);

    // Zapis nagłówka z wagami
    let encoded_freq = encode_frequencies(&freq, block_size as u8, original_len);
    let encoded_data = encode_data(&chunks, &table);

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
         ⚙️  Order:       {} (Block size: {})\n\
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