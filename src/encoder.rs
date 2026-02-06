mod huffman;

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::collections::HashMap;

// Jeśli używasz log, upewnij się, że są w Cargo.toml, w przeciwnym razie usuń te linie
// use log::{debug, error, info}; 
// Dla uproszczenia w tym przykładzie użyję println!

use crate::huffman::{
    CodeTable, FreqTable, build_code_table, build_huffman_tree, entropy_from_freq,
};

type MarkovFreqTable = HashMap<Vec<u8>, FreqTable>;
type MarkovCodeTable = HashMap<Vec<u8>, CodeTable>;

fn encode_frequencies(m_frequencies: &MarkovFreqTable, order: u8, original_len: u64) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&original_len.to_be_bytes());
    bytes.push(order);
    bytes.extend_from_slice(&(m_frequencies.len() as u32).to_be_bytes());

    for (context, f_table) in m_frequencies {
        bytes.extend_from_slice(context);
        bytes.extend_from_slice(&(f_table.len() as u32).to_be_bytes());

        for (symbol, freq) in f_table {
            bytes.push(symbol[0]);
            bytes.extend_from_slice(&freq.to_be_bytes());
        }
    }
    bytes
}

fn encode_data(raw_data: &[u8], m_code_table: &MarkovCodeTable, order: usize) -> Vec<u8> {
    let mut result = Vec::new();
    let mut current_byte = 0u8;
    let mut bit_count = 0;
    let mut context = vec![0u8; order];

    for &byte in raw_data {
        let codes = m_code_table.get(&context)
            .expect("Błąd krytyczny: Kontekst nie znaleziony (nie powinno się zdarzyć)");
        
        let symbol_to_encode = vec![byte];
        
        // Tutaj symbol musi istnieć, bo budowaliśmy drzewo na podstawie tych danych
        let code = codes.get(&symbol_to_encode)
            .expect("Błąd krytyczny: Symbol nie ma kodu");

        for bit_char in code.chars() {
            let bit = if bit_char == '1' { 1 } else { 0 };
            current_byte = (current_byte << 1) | bit;
            bit_count += 1;

            if bit_count == 8 {
                result.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }

        if order > 0 {
            context.remove(0);
            context.push(byte);
        }
    }

    // Dopełnienie zerami do pełnego bajtu
    if bit_count > 0 {
        result.push(current_byte << (8 - bit_count));
    }

    result
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Użycie: {} <input> [output] [--order=N]", args[0]);
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let mut output_filepath = "output.huff".to_string();
    let mut order = 0usize;

    for arg in &args[2..] {
        if arg.starts_with("--order=") {
            if let Ok(n) = arg.trim_start_matches("--order=").parse::<usize>() {
                order = n;
            }
        } else {
            output_filepath = arg.clone();
        }
    }

    // Ograniczenie rzędu, żeby nie przepełnić bufora w nagłówku (format zakłada 1 bajt na rząd)
    if order > 255 {
        println!("Ostrzeżenie: Maksymalny rząd to 255. Ustawiono na 255.");
        order = 255;
    }

    let raw_data = fs::read(input_filepath).expect("Błąd odczytu pliku");
    let original_len = raw_data.len() as u64;

    if original_len == 0 {
        println!("Plik jest pusty.");
        return;
    }

    // 1. Zbieranie statystyk
    let mut markov_freqs = MarkovFreqTable::new();
    let mut context = vec![0u8; order];

    for &byte in &raw_data {
        let f_table = markov_freqs.entry(context.clone()).or_insert_with(FreqTable::new);
        *f_table.entry(vec![byte]).or_insert(0) += 1;

        if order > 0 {
            context.remove(0);
            context.push(byte);
        }
    }

    // 2. Budowa drzew Huffmana
    let mut markov_codes = MarkovCodeTable::new();
    let mut weighted_entropy = 0.0;
    
    for (ctx, f_table) in &markov_freqs {
        let tree = build_huffman_tree(f_table).expect("Błąd budowy drzewa");
        let mut codes = CodeTable::new();
        build_code_table(&tree, String::new(), &mut codes);
        
        let ctx_count: u64 = f_table.values().sum();
        let prob_ctx = ctx_count as f64 / original_len as f64;
        weighted_entropy += prob_ctx * entropy_from_freq(f_table);
        
        markov_codes.insert(ctx.clone(), codes);
    }

    // 3. Kodowanie
    let encoded_header = encode_frequencies(&markov_freqs, order as u8, original_len);
    let encoded_data = encode_data(&raw_data, &markov_codes, order);

    // 4. Zapis
    let mut file = File::create(&output_filepath).expect("Błąd zapisu");
    file.write_all(&encoded_header).unwrap();
    file.write_all(&encoded_data).unwrap();

    let total_size = encoded_header.len() + encoded_data.len();
    println!(
        "\r\n✅ Kodowanie rzędu {} zakończone.\n\
         📂 Rozmiar nagłówka:  {} bajtów\n\
         💾 Rozmiar strumienia: {} bajtów\n\
         📊 Entropia H(X|C):   {:.4} bitów/symbol\n\
         🗜️  Kompresja:        {:.2}%",
        order, 
        encoded_header.len(), 
        encoded_data.len(), 
        weighted_entropy,
        100.0 * (1.0 - (total_size as f64 / original_len as f64))
    );
}