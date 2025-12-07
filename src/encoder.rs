mod huffman;

use std::collections::BinaryHeap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use log::{debug, error, info, trace, warn};

use crate::huffman::{
    CodeTable, FreqTable, Node, build_code_table, build_huffman_tree, entropy_from_freq,
};

fn encode_frequencies(frequencies: &FreqTable) -> Vec<u8> {
    debug!("Generating frequency header...");
    let mut bytes = Vec::new();

    let mut heap = BinaryHeap::new();
    for (&byte, &freq) in frequencies {
        heap.push(Node::Leaf { byte, freq });
    }

    let unique_symbols = heap.len();
    debug!("Unique symbols to encode: {}", unique_symbols);

    loop {
        let curr_most_freq_node = heap.pop();
        match curr_most_freq_node {
            Some(Node::Leaf { byte, freq }) => {
                bytes.extend_from_slice(&byte.to_be_bytes());
                trace!(
                    "Encoded frequency entry: byte {:#04x} ('{}') with frequency {} (original frequency {})",
                    byte,
                    byte as char,
                    unique_symbols - heap.len() - 1,
                    freq
                );
            }
            Some(Node::Internal { .. }) => {
                // Should not happen in frequency encoding
            }
            None => break,
        }
    }

    if unique_symbols == 0 {
        warn!("Frequency table is empty.");
        return bytes;
    }

    // no table with 0 symbols exists so we store count-1
    bytes.insert(0, (unique_symbols - 1) as u8);

    debug!(
        "Frequency header generated with {} symbols. Size: {} bytes",
        unique_symbols,
        bytes.len()
    );

    bytes
}

fn encode_data(data: &[u8], code_table: &CodeTable) -> Vec<u8> {
    debug!("Starting data encoding (bit packing)...");
    let start = Instant::now();

    // Note: A Vec<u8> to store bits is memory inefficient,
    // but preserving original logic structure for this example.
    let mut bits = Vec::with_capacity(data.len() * 8);

    for &b in data {
        if let Some(code) = code_table.get(&b) {
            for c in code.chars() {
                bits.push(if c == '1' { 1 } else { 0 });
            }
        } else {
            error!(
                "CRITICAL: Byte {:#04x} found in data but not in code table!",
                b
            );
        }
    }

    let raw_bit_count = bits.len();
    trace!("Total raw bits generated: {}", raw_bit_count);

    let mut padding_count = 0;
    while bits.len() % 8 != 0 {
        bits.push(0);
        padding_count += 1;
    }
    if padding_count > 0 {
        trace!(
            "Added {} bits of padding for byte alignment.",
            padding_count
        );
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
        "Data encoding finished in {:.2?}. Final size: {} bytes",
        start.elapsed(),
        bytes.len()
    );
    bytes
}

fn write_frequencies_and_data_to_file(
    filepath: &str,
    freq_encoded: &[u8],
    data_encoded: &[u8],
) -> std::io::Result<()> {
    info!("Writing output to file: {}", filepath);
    let mut file = File::create(filepath)?;

    file.write_all(freq_encoded)?;
    debug!("Wrote frequency header ({} bytes).", freq_encoded.len());

    file.write_all(data_encoded)?;
    debug!("Wrote encoded body ({} bytes).", data_encoded.len());

    Ok(())
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        error!("Usage: {} <input_file> [output_file]", args[0]);
        eprintln!("  📂 <input_file>:  path to the file to encode.");
        eprintln!("  💾 [output_file]: optional. Path to write the encoded output.");
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let output_filepath = args.get(2).map_or("output.huff", |s| s.as_str());

    info!("--- Start Encoding ---");
    let total_start = Instant::now();

    info!("Reading input file: {}", input_filepath);
    let data = fs::read(input_filepath).expect("cannot read input file");
    debug!("Input size: {} bytes", data.len());

    let mut freq = FreqTable::new();
    for &b in &data {
        *freq.entry(b).or_insert(0) += 1;
    }
    debug!("Frequency analysis complete.");

    let tree = build_huffman_tree(&freq).expect("could not build huffman tree");
    debug!("Huffman tree built.");

    let mut table = CodeTable::new();
    build_code_table(&tree, String::new(), &mut table);
    debug!("Code table generated. Entries: {}", table.len());

    let encoded_freq = encode_frequencies(&freq);
    let encoded_data = encode_data(&data, &table);

    if let Err(e) =
        write_frequencies_and_data_to_file(output_filepath, &encoded_freq, &encoded_data)
    {
        error!("Failed to write encoded file: {}", e);
        std::process::exit(1);
    }

    let input_size = data.len();
    let total_output_size = encoded_freq.len() + encoded_data.len();

    let file_entropy = entropy_from_freq(&freq);
    let compression_ratio = if input_size > 0 {
        100.0 * (1.0 - (total_output_size as f64) / (input_size as f64))
    } else {
        0.0
    };

    println!(
        "\r\n✅ encoding successful.\n\
         📂 input file:  {} ({} bytes)\n\
         💾 output file: {} ({} bytes)\n\
         ℹ️ entropy:     {:.2} bits/symbol\n\
         🗜️ compressed:  {:.2}%",
        input_filepath,
        input_size,
        output_filepath,
        total_output_size,
        file_entropy,
        compression_ratio
    );

    info!("Finished in {:.2?}", total_start.elapsed());
}
