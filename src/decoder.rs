mod huffman;

use crate::huffman::{
    build_code_table, build_huffman_tree, entropy_from_freq, CodeTable, FreqTable,
};
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

struct HeaderInfo {
    original_len: u64,
    block_size: usize,
    freq_table: FreqTable,
    data_start_offset: usize,
}

fn read_and_parse_header(content: &[u8]) -> std::io::Result<HeaderInfo> {
    // Struktura nagłówka:
    // [0..8]   Original Length (u64)
    // [8]      Block Size (u8)
    // [9..13]  Table Entries Count (u32)
    // [13..]   Symbols (Count * Block Size)
    
    if content.len() < 13 {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "File too short for header"));
    }

    let mut buf8 = [0u8; 8];
    buf8.copy_from_slice(&content[0..8]);
    let original_len = u64::from_be_bytes(buf8);

    let block_size = content[8] as usize;
    if block_size == 0 {
         return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Block size is zero"));
    }

    let mut buf4 = [0u8; 4];
    buf4.copy_from_slice(&content[9..13]);
    let table_entries = u32::from_be_bytes(buf4) as usize;

    debug!("Header Info: OrigLen={}, BlockSize={}, TableEntries={}", original_len, block_size, table_entries);

    let symbols_start = 13;
    let symbols_end = symbols_start + (table_entries * block_size);

    if symbols_end > content.len() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Header says table is larger than file"));
    }

    let mut freq = HashMap::new();
    let symbols_slice = &content[symbols_start..symbols_end];

    // Rekonstrukcja wag (sztuczna, zgodna z encoderem: 1..N)
    for (i, chunk) in symbols_slice.chunks(block_size).enumerate() {
        let symbol = chunk.to_vec();
        freq.insert(symbol, (i + 1) as u64);
    }

    Ok(HeaderInfo {
        original_len,
        block_size,
        freq_table: freq,
        data_start_offset: symbols_end,
    })
}

fn decode_data(encoded: &[u8], code_table: &CodeTable, block_size: usize) -> Vec<u8> {
    debug!("Starting bitstream decoding...");
    let start_time = Instant::now();

    // Konwersja bajtów na bity
    let mut bits = Vec::with_capacity(encoded.len() * 8);
    for &byte in encoded {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }

    let mut result_bytes = Vec::new();
    let mut current_code = String::new();

    // Odwrócona tabela: Code String -> Symbol (Vec<u8>)
    let reverse_table: HashMap<String, Vec<u8>> =
        code_table.iter().map(|(sym, code)| (code.clone(), sym.clone())).collect();

    for &bit in &bits {
        current_code.push(if bit == 1 { '1' } else { '0' });
        if let Some(symbol) = reverse_table.get(&current_code) {
            result_bytes.extend_from_slice(symbol);
            current_code.clear();
        }
    }

    debug!("Decoding loop finished in {:.2?}. Raw decoded size: {}", start_time.elapsed(), result_bytes.len());
    result_bytes
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        error!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_filepath = &args[1];
    let output_filepath = &args[2];

    info!("--- Start Decoding ---");
    let content = fs::read(input_filepath).expect("Failed to read input file");

    let header_info = match read_and_parse_header(&content) {
        Ok(h) => h,
        Err(e) => {
            error!("Header parse error: {}", e);
            std::process::exit(1);
        }
    };

    let tree = build_huffman_tree(&header_info.freq_table).expect("Empty tree");
    let mut code_table = HashMap::new();
    build_code_table(&tree, String::new(), &mut code_table);

    let data_slice = &content[header_info.data_start_offset..];
    let mut decoded_raw = decode_data(data_slice, &code_table, header_info.block_size);

    // Przycinanie paddingu
    if decoded_raw.len() as u64 > header_info.original_len {
        debug!("Trimming padding: {} -> {}", decoded_raw.len(), header_info.original_len);
        decoded_raw.truncate(header_info.original_len as usize);
    }

    let mut out_file = File::create(output_filepath).expect("Cannot create output file");
    out_file.write_all(&decoded_raw).expect("Write failed");

    println!(
        "\r\n✅ Decoding successful.\n\
         📂 Input:  {}\n\
         💾 Output: {} ({} bytes restored)",
        input_filepath, output_filepath, decoded_raw.len()
    );
}