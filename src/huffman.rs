use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

pub type CodeTable = HashMap<u8, String>;
pub type FreqTable = HashMap<u8, u64>;

#[derive(Debug, Eq, PartialEq)]
pub enum Node {
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


pub fn entropy_from_freq(freq: &FreqTable) -> f64 { #![allow(dead_code)]
    let total: u64 = freq.values().sum();
    let total_f = total as f64;

    freq.values()
        .map(|&count| {
            let p = count as f64 / total_f;
            -p * p.log2()
        })
        .sum()
}

pub fn build_huffman_tree(frequencies: &FreqTable) -> Option<Box<HuffmanTree>> {
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

pub fn build_code_table(node: &Node, prefix: String, table: &mut CodeTable) {
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