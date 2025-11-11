use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::fs;

#[derive(Debug, Eq, PartialEq)]
enum HuffmanNode {
    Leaf { byte: u8, freq: u64 },
    Internal { left: Box<HuffmanNode>, right: Box<HuffmanNode> },
    Terminal
}

impl Ord for HuffmanNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match self {
            HuffmanNode::Leaf { freq, .. } => match other {
                HuffmanNode::Leaf { byte: _ob, freq: of } => freq.cmp(of),
                HuffmanNode::Internal { .. } => Ordering::Less,           
                HuffmanNode::Terminal => Ordering::Less,    
            }
            HuffmanNode::Internal { .. } => Ordering::Greater,
            HuffmanNode::Terminal => Ordering::Equal,
        }
    }
}
impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for HuffmanNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HuffmanNode::Leaf { byte, freq } => write!(f, "Leaf: byte={} freq={}", byte, freq),
            HuffmanNode::Internal { .. } => write!(f, "Internal"),
            HuffmanNode::Terminal => write!(f, "Terminal"),
        }
    }
}

fn main() {
    let data = fs::read("data/text/simple.txt").expect("Failed to read file");

    println!("Read {} bytes from file.", data.len());
    let mut frequencies: [u64; 255] = [0; 255];
    for byte in &data {
        frequencies[*byte as usize] += 1;
    }

    let mut frequencies_heap = BinaryHeap::new();
    for (index, freq) in frequencies.iter().enumerate() {
        if freq > &0 {
            frequencies_heap.push(HuffmanNode::Leaf { byte: index as u8, freq: *freq });
        }
    }
    if frequencies_heap.is_empty() {
        println!("No data to process.");
        return;
    }

    let mut hufman_tree = HuffmanNode::Internal { left: Box::new(frequencies_heap.pop().unwrap()), right: Box::new(HuffmanNode::Terminal) };
    let mut current_node = &mut hufman_tree;    
    while let Some(freq) = frequencies_heap.pop() {
        let new_node = Box::new(HuffmanNode::Internal {
            left: Box::new(freq),
            right: Box::new(HuffmanNode::Terminal),
        });
                
    }
}
