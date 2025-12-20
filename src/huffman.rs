use log::{debug, trace};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

// ZMIANA: Symbol to teraz wektor bajtów, a nie pojedynczy u8
pub type Symbol = Vec<u8>;
pub type CodeTable = HashMap<Symbol, String>;
pub type FreqTable = HashMap<Symbol, u64>;

#[derive(Debug, Eq, PartialEq)]
pub enum Node {
    Leaf {
        symbol: Symbol, // ZMIANA: byte -> symbol (Vec<u8>)
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
            // Porównywanie wektorów (leksykograficzne) jest wbudowane w Rust
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

    debug!(
        "Calculated entropy: {:.4} bits/symbol (Total samples: {})",
        entropy, total
    );
    entropy
}

pub fn build_huffman_tree(frequencies: &FreqTable) -> Option<Box<HuffmanTree>> {
    debug!(
        "Building Huffman Tree from {} unique symbols",
        frequencies.len()
    );

    let mut freq_vec: Vec<_> = frequencies.iter().collect();

    freq_vec.sort_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)));

    let mut heap = BinaryHeap::new();

    for (i, (symbol, _freq)) in freq_vec.iter().enumerate() {
        let freq = (i + 1) as u64;

        heap.push(HeapNode {
            freq,
            node: Box::new(Node::Leaf {
                symbol: symbol.to_vec(),
                freq,
            }),
        });
    }
    trace!("Initial heap size: {}", heap.len());

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
            trace!("Assigning code to symbol {:?} : '{}'", symbol, prefix);
            table.insert(symbol.clone(), prefix);
        }
        Node::Internal { left, right, .. } => {
            build_code_table(left, format!("{}0", prefix), table);
            build_code_table(right, format!("{}1", prefix), table);
        }
    }
}