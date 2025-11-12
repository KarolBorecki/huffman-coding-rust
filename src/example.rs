use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::fs;

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
        // Reverse for min-heap
        other.freq.cmp(&self.freq)
    }
}

impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn build_huffman_tree(frequencies: &HashMap<u8, u64>) -> Option<Box<Node>> {
    let mut heap = BinaryHeap::new();

    // Fill the heap with leaf nodes
    for (&byte, &freq) in frequencies {
        heap.push(HeapNode {
            freq,
            node: Box::new(Node::Leaf { byte, freq }),
        });
    }

    // Combine nodes until only one remains
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

fn main() {
    // 1. Read file
    let data = fs::read("input.txt").expect("Failed to read file");

    // 2. Count frequencies
    let mut frequencies = HashMap::new();
    for &byte in &data {
        *frequencies.entry(byte).or_insert(0) += 1;
    }

    // 3. Build Huffman tree
    let tree = build_huffman_tree(&frequencies).expect("Failed to build tree");

    // 4. Generate codes
    let mut code_table = HashMap::new();
    build_code_table(&tree, String::new(), &mut code_table);

    // 5. Print results
    println!("Huffman Codes:");
    for (byte, code) in &code_table {
        if *byte == b'\n' {
            println!("'\\n' => {}", code);
        } else {
            println!("'{}' => {}", *byte as char, code);
        }
    }
}
