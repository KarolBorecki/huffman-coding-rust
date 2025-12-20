mod huffman;

use crate::huffman::{
    build_code_table, build_huffman_tree, CodeTable, FreqTable,
};
use log::{debug, error, info};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;

struct HeaderInfo {
    original_len: u64,
    block_size: usize,
    freq_table: FreqTable,
    data_start_offset: usize,
}

fn read_and_parse_header(content: &[u8]) -> std::io::Result<HeaderInfo> {
    // Format: [OrigLen 8B] [BlkSize 1B] [Count 4B] ([Sym X B][Freq 8B])...
    if content.len() < 13 {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "File too short"));
    }

    let mut buf8 = [0u8; 8];
    buf8.copy_from_slice(&content[0..8]);
    let original_len = u64::from_be_bytes(buf8);

    let block_size = content[8] as usize;
    
    let mut buf4 = [0u8; 4];
    buf4.copy_from_slice(&content[9..13]);
    let table_entries = u32::from_be_bytes(buf4) as usize;

    debug!("Header: Entries={}, BlockSize={}", table_entries, block_size);

    // Każdy wpis to (block_size) bajtów symbolu + 8 bajtów wagi
    let entry_size = block_size + 8;
    let header_table_size = table_entries * entry_size;
    let symbols_start = 13;
    let symbols_end = symbols_start + header_table_size;

    if symbols_end > content.len() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Corrupt header size"));
    }

    let mut freq = HashMap::new();
    let table_slice = &content[symbols_start..symbols_end];

    for chunk in table_slice.chunks(entry_size) {
        // Podział: [Symbol ... | Waga (8B)]
        let symbol = chunk[0..block_size].to_vec();
        
        let mut freq_buf = [0u8; 8];
        freq_buf.copy_from_slice(&chunk[block_size..]);
        let weight = u64::from_be_bytes(freq_buf);

        freq.insert(symbol, weight);
    }

    Ok(HeaderInfo {
        original_len,
        block_size,
        freq_table: freq,
        data_start_offset: symbols_end,
    })
}

fn decode_data(encoded: &[u8], code_table: &CodeTable) -> Vec<u8> {
    let mut bits = Vec::with_capacity(encoded.len() * 8);
    for &byte in encoded {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }

    let mut result_bytes = Vec::new();
    let mut current_code = String::new();

    let reverse_table: HashMap<String, Vec<u8>> =
        code_table.iter().map(|(sym, code)| (code.clone(), sym.clone())).collect();

    for &bit in &bits {
        current_code.push(if bit == 1 { '1' } else { '0' });
        if let Some(symbol) = reverse_table.get(&current_code) {
            result_bytes.extend_from_slice(symbol);
            current_code.clear();
        }
    }
    result_bytes
}

fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        error!("Usage: decoder <input> <output>");
        std::process::exit(1);
    }
    let input_filepath = &args[1];
    let output_filepath = &args[2];
    let content = fs::read(input_filepath).expect("Read failed");
    let header = read_and_parse_header(&content).expect("Header parse failed");

    // Budowa drzewa na podstawie prawdziwych wag odczytanych z pliku
    let tree = build_huffman_tree(&header.freq_table).expect("Tree build failed");
    let mut table = HashMap::new();
    build_code_table(&tree, String::new(), &mut table);

    let mut decoded = decode_data(&content[header.data_start_offset..], &table);
    
    // Usunięcie paddingu
    if decoded.len() as u64 > header.original_len {
        decoded.truncate(header.original_len as usize);
    }

    fs::write(output_filepath, &decoded).expect("Write failed");
        println!(
        "\r\n✅ Decoding successful.\n\
         📂 Input:  {}\n\
         💾 Output: {} ({} bytes restored)",
        input_filepath, output_filepath, decoded.len()
    );
}