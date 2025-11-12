use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::Write;

#[derive(Debug, Eq, PartialEq)]
enum Node {
    Leaf { byte: u8, freq: u64 },
    Internal { freq: u64, left: Box<Node>, right: Box<Node> },
}

impl Node {
    fn freq(&self) -> u64 {
        match self {
            Node::Leaf { freq, .. } => *freq,
            Node::Internal { freq, .. } => *freq,
        }
    }
}

#[derive(Eq, PartialEq)]
struct HeapNode {
    freq: u64,
    node: Box<Node>,
}

impl Ord for HeapNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq.cmp(&self.freq) // min-heap
    }
}

impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------- BUILD TREE AND CODES ----------

fn build_huffman_tree(frequencies: &HashMap<u8, u64>) -> Option<Box<Node>> {
    let mut heap = BinaryHeap::new();
    for (&byte, &freq) in frequencies {
        heap.push(HeapNode {
            freq,
            node: Box::new(Node::Leaf { byte, freq }),
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

// ---------- BIT SERIALIZATION ----------

fn serialize_tree(node: &Node, bits: &mut Vec<u8>) {
    match node {
        Node::Leaf { byte, .. } => {
            bits.push(1);
            for i in (0..8).rev() {
                bits.push((byte >> i) & 1);
            }
        }
        Node::Internal { left, right, .. } => {
            serialize_tree(left, bits);
            serialize_tree(right, bits);
            bits.push(0);
        }
    }
}

fn encode_with_tree(data: &[u8], code_table: &HashMap<u8, String>, tree: &Node) -> Vec<u8> {
    let mut bits = Vec::new();

    // 1️⃣ tree
    serialize_tree(tree, &mut bits);
    // 2️⃣ separator
    bits.extend(vec![0; 16]);
    // 3️⃣ data bits
    for &b in data {
        if let Some(code) = code_table.get(&b) {
            for c in code.chars() {
                bits.push(if c == '1' { 1 } else { 0 });
            }
        }
    }
    // 4️⃣ pad
    while bits.len() % 8 != 0 {
        bits.push(0);
    }
    // 5️⃣ to bytes
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

// ---------- DECODER ----------

/// Convert bytes to a vector of bits (u8 0/1)
fn bytes_to_bits(data: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data.len() * 8);
    for &byte in data {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    bits
}

/// Deserialize the tree from the bitstream (same structure as serialize_tree)
fn deserialize_tree(bits: &mut Vec<u8>, index: &mut usize) -> Option<Box<Node>> {
    if *index >= bits.len() {
        return None;
    }
    let flag = bits[*index];
    *index += 1;

    if flag == 1 {
        // leaf
        if *index + 8 > bits.len() {
            return None;
        }
        let mut byte = 0u8;
        for _ in 0..8 {
            byte = (byte << 1) | bits[*index];
            *index += 1;
        }
        Some(Box::new(Node::Leaf { byte, freq: 0 }))
    } else {
        // internal node
        let left = deserialize_tree(bits, index)?;
        let right = deserialize_tree(bits, index)?;
        Some(Box::new(Node::Internal {
            freq: 0,
            left,
            right,
        }))
    }
}

fn decode_with_tree(encoded: &[u8]) -> Vec<u8> {
    let bits = bytes_to_bits(encoded);

    // find the 16-bit separator
    let mut sep_pos = None;
    for i in 0..bits.len() - 15 {
        if bits[i..i + 16].iter().all(|&b| b == 0) {
            sep_pos = Some(i);
            break;
        }
    }
    let sep_pos = sep_pos.expect("separator not found");

    let mut idx = 0;
    let mut tree_bits = bits[..sep_pos].to_vec();
    let mut t_idx = 0;
    let tree = deserialize_tree(&mut tree_bits, &mut t_idx).expect("tree parse error");

    // start after separator
    let mut data_bits = bits[sep_pos + 16..].to_vec();
    let mut result = Vec::new();
    let mut node = &tree;

    for bit in data_bits {
        match node {
            Node::Leaf { byte, .. } => {
                result.push(*byte);
                node = &tree; // restart
            }
            Node::Internal { left, right, .. } => {
                node = if bit == 0 { left } else { right };
            }
        }
    }

    // last byte check
    if let Node::Leaf { byte, .. } = node {
        result.push(*byte);
    }

    result
}

// ---------- MAIN ----------

fn main() {
    // --- ENCODE ---
    let data = fs::read("input.txt").expect("cannot read input.txt");

    let mut freq = HashMap::new();
    for &b in &data {
        *freq.entry(b).or_insert(0) += 1;
    }

    let tree = build_huffman_tree(&freq).unwrap();
    let mut table = HashMap::new();
    build_code_table(&tree, String::new(), &mut table);

    let encoded = encode_with_tree(&data, &table, &tree);
    fs::write("output.huff", &encoded).expect("write failed");
    println!("✅ Encoded to output.huff ({} bytes)", encoded.len());

    // --- DECODE ---
    let encoded_data = fs::read("output.huff").expect("read failed");
    let decoded = decode_with_tree(&encoded_data);
    let mut f = File::create("decoded.txt").expect("create failed");
    f.write_all(&decoded).unwrap();
    println!("✅ Decoded to decoded.txt ({} bytes)", decoded.len());
}
