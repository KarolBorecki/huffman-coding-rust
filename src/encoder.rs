mod huffman;

use crate::huffman::{build_code_table, build_huffman_tree, entropy_from_freq, Node};
use std::collections::{BinaryHeap, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::Write;

fn encode_frequencies(frequencies: &HashMap<u8, u64>) -> Vec<u8> {
    let mut bytes = Vec::new();

    let mut heap = BinaryHeap::new();
    for (&byte, &freq) in frequencies {
        heap.push(Node::Leaf { byte, freq });
    }
    let mut count = 0;

    loop {
        let curr_most_freq_node = heap.pop() ;
        match curr_most_freq_node {
            Some(Node::Leaf { byte, freq: _ }) => {
                bytes.extend_from_slice(&byte.to_be_bytes());
                count += 1;
            }
            Some(Node::Internal { .. }) => {
                // Should not happen in frequency encoding
            }
            None => break,
        }
    }
    bytes.insert(0, count as u8);

    bytes
}

fn encode_data(data: &[u8], code_table: &HashMap<u8, String>) -> Vec<u8> {
    let mut bits = Vec::new();

    for &b in data {
        if let Some(code) = code_table.get(&b) {
            for c in code.chars() {
                bits.push(if c == '1' { 1 } else { 0 });
            }
        }
    }
    while bits.len() % 8 != 0 {
        bits.push(0);
    }
    let mut bytes = Vec::new();
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for &bit in chunk {
            byte = (byte << 1) | bit;
        }

        bytes.push(byte);
    }
    bytes
}

fn write_frequencies_and_data_to_file(
    filepath: &str,
    freq_encoded: &[u8],
    data_encoded: &[u8],
) -> std::io::Result<()> {
    let mut file = File::create(filepath)?;
    file.write_all(freq_encoded)?;
    file.write_all(data_encoded)?;
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <input_file> [output_file]", args[0]);
        eprintln!("  <input_file>:  Path to the file to encode.");
        eprintln!("  [output_file]: Optional. Path to write the encoded output.");
        eprintln!("                 Defaults to 'output.huff'.");
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let output_filepath = args.get(2).map_or("output.huff", |s| s.as_str());

    // ENCODER
    let data = fs::read(input_filepath).expect("cannot read input.txt");
    let mut freq = HashMap::new();
    for &b in &data {
        *freq.entry(b).or_insert(0) += 1;
    }
    let tree = build_huffman_tree(&freq).expect("could not build huffman tree");
    let mut table = HashMap::new();
    build_code_table(&tree, String::new(), &mut table);

    let encoded_freq = encode_frequencies(&freq);
    let encoded_data = encode_data(&data, &table);
    write_frequencies_and_data_to_file(output_filepath, &encoded_freq, &encoded_data)
        .expect("failed to write encoded file");
    println!(
        "✅ Encoded to \"{}\" file \"{}\" of entrophy {:.2} - down to {} bytes from {} bytes -{:.2} %",
        output_filepath,
        input_filepath,
        entropy_from_freq(&freq),
        encoded_data.len(),
        data.len(),
        (1.0 - (encoded_data.len() as f64) / (data.len() as f64)) * 100.0
    );
}
