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

// Implementacja Ord dla Node zapewnia determinizm przy porównywaniu węzłów o tej samej wadze
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        let freq_cmp = other.freq().cmp(&self.freq());
        if freq_cmp != Ordering::Equal {
            return freq_cmp;
        }

        match (self, other) {
            // Przy równych wagach, sortujemy leksykograficznie po symbolu
            (Node::Leaf { symbol: a, .. }, Node::Leaf { symbol: b, .. }) => a.cmp(b),
            // Liście mają pierwszeństwo przed węzłami wewnętrznymi (konwencja dla determinizmu)
            (Node::Leaf { .. }, Node::Internal { .. }) => Ordering::Less,
            (Node::Internal { .. }, Node::Leaf { .. }) => Ordering::Greater,
            // Dwa węzły wewnętrzne o tej samej wadze są "równe"
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

// KLUCZOWA POPRAWKA: Determinizm sterty
impl Ord for HeapNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap to MaxHeap, więc odwracamy kolejność częstotliwości (najmniejsze najpierw)
        other.freq.cmp(&self.freq)
            // JEŚLI CZĘSTOTLIWOŚCI SĄ RÓWNE: używamy porównania Node (leksykograficznie),
            // aby enkoder i dekoder zawsze podejmowały tę samą decyzję co do kolejności łączenia.
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn entropy_from_freq(freq: &FreqTable) -> f64 {
    let total: u64 = freq.values().sum();
    if total == 0 { return 0.0; }
    let total_f = total as f64;

    freq.values()
        .map(|&count| {
            if count == 0 { return 0.0; }
            let p = count as f64 / total_f;
            -p * p.log2()
        })
        .sum()
}

pub fn build_huffman_tree(frequencies: &FreqTable) -> Option<Box<HuffmanTree>> {
    if frequencies.is_empty() { return None; }

    let mut heap = BinaryHeap::new();

    for (symbol, freq) in frequencies {
        heap.push(HeapNode {
            freq: *freq,
            node: Box::new(Node::Leaf {
                symbol: symbol.to_vec(),
                freq: *freq,
            }),
        });
    }

    // POPRAWKA: Jeśli jest tylko jeden symbol, tworzymy sztuczny węzeł.
    // Używamy pustego wektora vec![], aby nie kolidował z prawdziwym symbolem [0] (null byte).
    if heap.len() == 1 {
        let only_node = heap.pop().unwrap();
        return Some(Box::new(Node::Internal {
            freq: only_node.freq,
            left: only_node.node,
            right: Box::new(Node::Leaf { symbol: vec![], freq: 0 }), 
        }));
    }

    while heap.len() > 1 {
        let left = heap.pop().unwrap();
        let right = heap.pop().unwrap();
        let freq = left.freq + right.freq;
        heap.push(HeapNode {
            freq,
            node: Box::new(Node::Internal {
                freq,
                left: left.node,
                right: right.node,
            }),
        });
    }

    heap.pop().map(|n| n.node)
}

pub fn build_code_table(node: &Node, prefix: String, table: &mut CodeTable) {
    match node {
        Node::Leaf { symbol, freq } => {
            // Ignorujemy dummy node (freq 0), żeby nie śmiecić w tabeli kodów
            // oraz puste wektory
            if *freq > 0 || !symbol.is_empty() {
                table.insert(symbol.clone(), prefix);
            }
        }
        Node::Internal { left, right, .. } => {
            build_code_table(left, format!("{}0", prefix), table);
            build_code_table(right, format!("{}1", prefix), table);
        }
    }
}