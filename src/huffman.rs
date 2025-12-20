use log::{debug, trace};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

pub type Symbol = Vec<u8>;
pub type CodeTable = HashMap<Symbol, String>;
pub type FreqTable = HashMap<Symbol, u64>;

#[derive(Debug, Eq, PartialEq)]
pub enum Node {
    Leaf {
        symbol: Symbol,
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
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        let freq_cmp = other.freq().cmp(&self.freq());
        if freq_cmp != Ordering::Equal {
            return freq_cmp;
        }

        match (self, other) {
            (Node::Leaf { symbol: a, .. }, Node::Leaf { symbol: b, .. }) => a.cmp(b),
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

pub type HuffmanTree = Node;

#[derive(Eq, PartialEq)]
pub struct HeapNode {
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

pub fn entropy_from_freq(freq: &FreqTable) -> f64 {
    let total: u64 = freq.values().sum();
    let total_f = total as f64;

    let entropy: f64 = freq
        .values()
        .map(|&count| {
            let p = count as f64 / total_f;
            -p * p.log2()
        })
        .sum();
    entropy
}

pub fn build_huffman_tree(frequencies: &FreqTable) -> Option<Box<HuffmanTree>> {
    debug!("Building Huffman Tree from {} unique symbols", frequencies.len());

    let mut freq_vec: Vec<(&Symbol, u64)> = frequencies
        .iter()
        .map(|(sym, freq)| (sym, *freq))
        .collect();

    let limit = u64::MAX / 2;
    let mut total_weight: u128 = freq_vec.iter().map(|(_, f)| *f as u128).sum();

    if total_weight > limit as u128 {
        debug!(
            "⚠️ Total weight ({}) exceeds safety limit. Normalizing weights...",
            total_weight
        );

        while total_weight > limit as u128 {
            total_weight = 0;
            for (_, freq) in freq_vec.iter_mut() {
                *freq = (*freq >> 1).max(1);
                total_weight += *freq as u128;
            }
        }
        debug!("Weights normalized. New total: {}", total_weight);
    }

    freq_vec.sort_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)));

    let mut heap = BinaryHeap::new();

    for (_i, (symbol, freq)) in freq_vec.iter().enumerate() {
        heap.push(HeapNode {
            freq: *freq,
            node: Box::new(Node::Leaf {
                symbol: symbol.to_vec(),
                freq: *freq,
            }),
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

    debug!("Tree construction complete.");
    heap.pop().map(|n| n.node)
}

pub fn build_code_table(node: &Node, prefix: String, table: &mut CodeTable) {
    match node {
        Node::Leaf { symbol, .. } => {
            table.insert(symbol.clone(), prefix);
        }
        Node::Internal { left, right, .. } => {
            build_code_table(left, format!("{}0", prefix), table);
            build_code_table(right, format!("{}1", prefix), table);
        }
    }
}