use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs;

#[derive(Clone, Debug, Eq, PartialEq)]
enum HuffmanNode {
    Leaf { byte: u8, freq: u64 },
    Internal { left: Box<HuffmanNode>, right: Box<HuffmanNode>, freq: u64 },
}

impl HuffmanNode {
    fn freq(&self) -> u64 {
        match self {
            HuffmanNode::Leaf { freq, .. } => *freq,
            HuffmanNode::Internal { freq, .. } => *freq,
        }
    }
}

impl Ord for HuffmanNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq().cmp(&self.freq()) 
    }
}

impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn build_huffman_code_table(root: &HuffmanNode, prefix: Vec<bool>, table: &mut Vec<(u8, Vec<bool>)>) {
    match root {
        HuffmanNode::Leaf { byte, .. } => {
            table.push((*byte, prefix));
        }
        HuffmanNode::Internal { left, right, .. } => {
            let mut left_prefix = prefix.clone();
            left_prefix.push(false);
            build_huffman_code_table(left, left_prefix, table);

            let mut right_prefix = prefix;
            right_prefix.push(true);
            build_huffman_code_table(right, right_prefix, table);
        }
    }
}

fn main() {
    let data = fs::read("data/text/simple.txt").expect("Failed to read file");

    println!("Read {} bytes from file.", data.len());
    let mut frequencies: [u64; 256] = [0; 256];
    for &byte in &data {
        frequencies[byte as usize] += 1;
    }

    let mut heap = BinaryHeap::new();
    for (byte, &freq) in frequencies.iter().enumerate() {
        if freq > 0 {
            heap.push(HuffmanNode::Leaf {
                byte: byte as u8,
                freq,
            });
        }
    }

    if heap.is_empty() {
        println!("No data to process.");
        return;
    }

    while heap.len() > 1 {
        let node1 = heap.pop().unwrap();
        let node2 = heap.pop().unwrap();

        let new_node = HuffmanNode::Internal {
            left: Box::new(node1.clone()),
            right: Box::new(node2.clone()),
            freq: node1.freq() + node2.freq(),
        };

        heap.push(new_node);
    }

    let root = heap.pop().unwrap();
    println!("Zbudowano drzewo Huffmana: {:?}", root);

    let mut code_table = Vec::new();
    build_huffman_code_table(&root, Vec::new(), &mut code_table);

    println!("Tabela kodów Huffmana:");
    for (byte, code) in code_table {
        println!("Bajt: {:02x}, Kod: {:?}", byte, code);
    }
}
