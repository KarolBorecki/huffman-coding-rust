use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::Write;

#[derive(Debug, Eq, PartialEq)]
enum Node {
    Leaf {
        byte: u8,
        freq: u64,
    },
    Internal {
        freq: u64,
        left: Box<Node>,
        right: Box<Node>,
    },
}

impl Node {
    fn freq(&self) -> u64 {
        match self {
            Node::Leaf { freq, .. } => *freq,
            Node::Internal { freq, .. } => *freq,
        }
    }

    fn character(&self) -> Option<u8> {
        match self {
            Node::Leaf { byte, .. } => Some(*byte),
            Node::Internal { .. } => None,
        }
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        let freq_cmp = other.freq().cmp(&self.freq());
        if freq_cmp != Ordering::Equal {
            return freq_cmp;
        }

        match (self, other) {
            (Node::Leaf { byte: a, .. }, Node::Leaf { byte: b, .. }) => a.cmp(b),
            (Node::Leaf { .. }, Node::Internal { .. }) => Ordering::Less,
            (Node::Internal { .. }, Node::Leaf { .. }) => Ordering::Greater,
            (Node::Internal { .. }, Node::Internal { .. }) => Ordering::Equal,
        }
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

type HuffmanTree = Node;

#[derive(Eq, PartialEq)]
struct HeapNode {
    freq: u64,
    node: Box<Node>,
}

impl Ord for HeapNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq.cmp(&self.freq)
    }
}

impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn build_huffman_tree(frequencies: &HashMap<u8, u64>) -> Option<Box<HuffmanTree>> {
    let mut freq_vec: Vec<_> = frequencies.iter().collect();
    freq_vec.sort_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)));

    let mut heap = BinaryHeap::new();
    for (i, (byte, _freq)) in freq_vec.iter().enumerate() {
        let freq = (i + 1) as u64;
        heap.push(HeapNode {
            freq,
            node: Box::new(Node::Leaf { byte: **byte, freq }),
        });
    }

    while heap.len() > 1 {
        let left = heap.pop().unwrap();
        let right = heap.pop().unwrap();
        let freq = left.freq + right.freq;
        let new_node = Node::Internal {
            freq,
            left: left.node,
            right: right.node,
        };
        heap.push(HeapNode {
            freq,
            node: Box::new(new_node),
        });
    }
    heap.pop().map(|n| n.node)
}

fn build_code_table(node: &Node, prefix: String, table: &mut HashMap<u8, String>) {
    match node {
        Node::Leaf { byte, .. } => {
            table.insert(*byte, prefix);
        }
        Node::Internal { left, right, .. } => {
            build_code_table(left, format!("{}0", prefix), table);
            build_code_table(right, format!("{}1", prefix), table);
        }
    }
}

fn encode_frequencies(frequencies: &HashMap<u8, u64>) -> Vec<u8> {
    let mut bytes = Vec::new();

    let mut heap = BinaryHeap::new();
    for (&byte, &freq) in frequencies {
        heap.push(Node::Leaf { byte, freq });
    }
    let mut count = 0;

    while let curr_most_freq_node = heap.pop() {
        match curr_most_freq_node {
            Some(Node::Leaf { byte, freq }) => {
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
        byte_index += 1;
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

fn read_frequencies_and_data_from_file(filepath: &str) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    let content = fs::read(filepath)?;
    let freq_size = content[0] as usize + 1;
    let freq_encoded = content[..freq_size].to_vec();
    let data_encoded = content[freq_size..].to_vec();
    Ok((freq_encoded, data_encoded))
}

fn decode_frequencies(encoded: &[u8]) -> HashMap<u8, u64> {
    let mut freq = HashMap::new();
    let count = encoded[0] as usize;
    for i in 0..count {
        let byte = encoded[i + 1];
        freq.insert(byte, (i + 1) as u64);
    }
    freq
}

fn decode_data(encoded: &[u8], code_table: &HashMap<u8, String>) -> Vec<u8> {
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
        "✅ Encoded to {} (down to {} bytes from {} bytes -{} %)\n\n",
        output_filepath,
        encoded_data.len(),
        data.len(),
        (1.0 - (encoded_data.len() as f64) / (data.len() as f64)) * 100.0
    );

    // DECODER
    let (encoded_freq, encoded_data) =
        read_frequencies_and_data_from_file(output_filepath).expect("failed to read encoded file");

    let decoded_freq = decode_frequencies(&encoded_freq);

    let decoded_tree = build_huffman_tree(&decoded_freq).unwrap();

    let mut decoded_table = HashMap::new();
    build_code_table(&decoded_tree, String::new(), &mut decoded_table);

    let decoded_data = decode_data(&encoded_data, &decoded_table);
    assert_eq!(data, decoded_data);
    let decoded_output_filepath = String::from(format!("{}.decoded", input_filepath));
    let mut decoded_output_file =
        File::create(decoded_output_filepath).expect("cannot create decoded_output file");
    decoded_output_file
        .write_all(&decoded_data)
        .expect("could not write decoded data");
    println!("✅ Decoding successful, data matches original.");
}
