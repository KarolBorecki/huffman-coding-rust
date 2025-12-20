mod huffman;

use std::collections::BinaryHeap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use log::{debug, error, info, trace, warn};

use crate::huffman::{
    build_code_table, build_huffman_tree, entropy_from_freq, CodeTable, FreqTable, Node, Symbol,
};

fn encode_frequencies(frequencies: &FreqTable, block_size: u8, original_len: u64) -> Vec<u8> {
    debug!("Generating frequency header...");
    let mut bytes = Vec::new();

    // 1. Zapisz oryginalną długość pliku (8 bajtów) - aby decoder wiedział gdzie uciąć padding
    bytes.extend_from_slice(&original_len.to_be_bytes());

    // 2. Zapisz rozmiar bloku (1 bajt)
    bytes.push(block_size);

    let mut heap = BinaryHeap::new();
    for (symbol, &freq) in frequencies {
        heap.push(Node::Leaf {
            symbol: symbol.clone(),
            freq,
        });
    }

    let unique_symbols = heap.len();
    debug!("Unique symbols to encode: {}", unique_symbols);

    // 3. Zapisz liczbę symboli w tabeli (4 bajty - u32, bo przy rzędzie 2 może być ich dużo)
    bytes.extend_from_slice(&(unique_symbols as u32).to_be_bytes());

    // Zapisujemy symbole w kolejności od najczęstszego (według logiki sortowania z huffman.rs)
    loop {
        let curr_most_freq_node = heap.pop();
        match curr_most_freq_node {
            Some(Node::Leaf { symbol, .. }) => {
                // Każdy symbol ma długość 'block_size'
                bytes.extend_from_slice(&symbol);
            }
            Some(Node::Internal { .. }) => {}
            None => break,
        }
    }

    debug!(
        "Header generated. Total header size: {} bytes",
        bytes.len()
    );

    bytes
}

fn encode_data(data_blocks: &[Vec<u8>], code_table: &CodeTable) -> Vec<u8> {
    debug!("Starting data encoding (bit packing)...");
    let start = Instant::now();

    let mut bits = Vec::with_capacity(data_blocks.len() * 8); // Przybliżenie

    for block in data_blocks {
        if let Some(code) = code_table.get(block) {
            for c in code.chars() {
                bits.push(if c == '1' { 1 } else { 0 });
            }
        } else {
            error!("CRITICAL: Symbol {:?} found in data but not in code table!", block);
        }
    }

    // Padding bitowy (dopełnienie do pełnego bajtu)
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

    debug!(
        "Data encoding finished in {:.2?}. Final body size: {} bytes",
        start.elapsed(),
        bytes.len()
    );
    bytes
}

fn write_output(
    filepath: &str,
    freq_encoded: &[u8],
    data_encoded: &[u8],
) -> std::io::Result<()> {
    info!("Writing output to file: {}", filepath);
    let mut file = File::create(filepath)?;
    file.write_all(freq_encoded)?;
    file.write_all(data_encoded)?;
    Ok(())
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        error!("Usage: {} <input_file> [output_file] [--order=N]", args[0]);
        eprintln!("  📂 <input_file>:  ścieżka do pliku wejściowego.");
        eprintln!("  💾 [output_file]: opcjonalnie. ścieżka wyjściowa.");
        eprintln!("  ⚙️  --order=N:     rząd modelowania (0, 1, 2). Domyślnie 0.");
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let mut output_filepath = "output.huff";
    let mut order = 0usize;

    // Proste parsowanie argumentów
    for arg in &args[2..] {
        if arg.starts_with("--order=") {
            if let Ok(n) = arg.trim_start_matches("--order=").parse::<usize>() {
                if n <= 2 {
                    order = n;
                } else {
                    warn!("Obsługiwane rzędy to 0, 1, 2. Ustawiono order=2.");
                    order = 2;
                }
            }
        } else {
            output_filepath = arg;
        }
    }

    let block_size = order + 1;
    info!("--- Start Encoding (Order: {}, BlockSize: {}) ---", order, block_size);
    let total_start = Instant::now();

    info!("Reading input file: {}", input_filepath);
    let raw_data = fs::read(input_filepath).expect("cannot read input file");
    let original_len = raw_data.len() as u64;
    debug!("Input size: {} bytes", original_len);

    let chunks: Vec<Symbol> = raw_data
        .chunks(block_size)
        .map(|chunk| {
            let mut c = chunk.to_vec();
            while c.len() < block_size {
                c.push(0); // dopełniamy zerami ostatni kawałek
            }
            c
        })
        .collect();
    
    debug!("Data split into {} blocks.", chunks.len());

    let mut freq = FreqTable::new();
    for block in &chunks {
        *freq.entry(block.clone()).or_insert(0) += 1;
    }
    debug!("Frequency analysis complete. Unique symbols: {}", freq.len());

    let tree = build_huffman_tree(&freq).expect("could not build huffman tree");
    
    let mut table = CodeTable::new();
    build_code_table(&tree, String::new(), &mut table);

    // Przekazujemy block_size i original_len do nagłówka
    let encoded_freq = encode_frequencies(&freq, block_size as u8, original_len);
    let encoded_data = encode_data(&chunks, &table);

    if let Err(e) = write_output(output_filepath, &encoded_freq, &encoded_data) {
        error!("Failed to write encoded file: {}", e);
        std::process::exit(1);
    }

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
         ⚙️  Order:       {} (Block size: {})\n\
         💾  Output:      {} ({} bytes)\n\
         ℹ️  Entropy:     {:.4} bits/symbol\n\
         🗜️  Ratio:       {:.4}%",
        input_filepath,
        original_len,
        order,
        block_size,
        output_filepath,
        total_output_size,
        file_entropy,
        compression_ratio
    );

    info!("Finished in {:.2?}", total_start.elapsed());
}