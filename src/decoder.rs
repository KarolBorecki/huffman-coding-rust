mod huffman;
use crate::huffman::{FreqTable, build_code_table, build_huffman_tree};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Cursor, Read};

struct HeaderInfo {
    original_len: u64,
    order: usize,
    markov_tables: HashMap<Vec<u8>, HashMap<String, u8>>,
    data_start_offset: usize,
}

fn read_and_parse_header(content: &[u8]) -> std::io::Result<HeaderInfo> {
    let mut cursor = Cursor::new(content);

    let mut buf8 = [0u8; 8];
    cursor.read_exact(&mut buf8)?;
    let original_len = u64::from_be_bytes(buf8);

    let mut buf1 = [0u8; 1];
    cursor.read_exact(&mut buf1)?;
    let order = buf1[0] as usize;

    let mut buf4 = [0u8; 4];
    cursor.read_exact(&mut buf4)?;
    let num_contexts = u32::from_be_bytes(buf4) as usize;

    let mut markov_tables = HashMap::new();

    for _ in 0..num_contexts {
        let mut context_key = vec![0u8; order];
        if order > 0 {
            cursor.read_exact(&mut context_key)?;
        }

        let mut sym_count_buf = [0u8; 4];
        cursor.read_exact(&mut sym_count_buf)?;
        let num_symbols = u32::from_be_bytes(sym_count_buf) as usize;

        let mut freq_table = FreqTable::new();
        for _ in 0..num_symbols {
            let mut sym_buf = [0u8; 1];
            cursor.read_exact(&mut sym_buf)?;
            let mut f_buf = [0u8; 8];
            cursor.read_exact(&mut f_buf)?;
            freq_table.insert(vec![sym_buf[0]], u64::from_be_bytes(f_buf));
        }

        let tree = build_huffman_tree(&freq_table).expect("Błąd drzewa");
        let mut code_table = HashMap::new();
        build_code_table(&tree, String::new(), &mut code_table);

        let mut reverse_table = HashMap::new();

        for (sym_vec, code_str) in code_table {
            // Filtrujemy dummy node (vec![]) oraz sprawdzamy obecność w freq_table
            if !sym_vec.is_empty() && freq_table.contains_key(&sym_vec) {
                reverse_table.insert(code_str, sym_vec[0]);
            }
        }
        markov_tables.insert(context_key, reverse_table);
    }

    let data_offset = cursor.position() as usize;
    Ok(HeaderInfo {
        original_len,
        order,
        markov_tables,
        data_start_offset: data_offset,
    })
}

fn decode_data(
    encoded: &[u8],
    markov_tables: &HashMap<Vec<u8>, HashMap<String, u8>>,
    order: usize,
    original_len: u64,
) -> Vec<u8> {
    let mut result = Vec::with_capacity(original_len as usize);
    let mut context = vec![0u8; order];
    let mut current_bit_string = String::new();

    let mut bit_iter = encoded
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1));

    // 1. Pobierz tabelę początkową RAZ przed pętlą
    let mut current_table = markov_tables
        .get(&context)
        .expect("Błąd: Nieznany kontekst startowy - plik uszkodzony");

    println!("Dostępne konteksty: {:?}", markov_tables.keys().collect::<Vec<_>>());
println!("Szukany kontekst startowy: {:?}", context);
    while (result.len() as u64) < original_len {
    let current_table = markov_tables.get(&context).expect("Błąd kontekstu");

    // 1. SPRAWDŹ, CZY SYMBOL JEST DETERMINISTYCZNY (kod "")
    // Jeśli w tabeli jest kod pusty, bierzemy go bez czytania bitów
    if let Some(&decoded_byte) = current_table.get("") {
        result.push(decoded_byte);
        if order > 0 {
            context.remove(0);
            context.push(decoded_byte);
        }
        current_bit_string.clear();
        continue; // Przejdź do kolejnego symbolu bez pobierania bitu
    }

    // 2. JEŚLI NIE, CZYTAJ BITY
    if let Some(bit) = bit_iter.next() {
        current_bit_string.push(if bit == 1 { '1' } else { '0' });

        if let Some(&decoded_byte) = current_table.get(&current_bit_string) {
            result.push(decoded_byte);
            if order > 0 {
                context.remove(0);
                context.push(decoded_byte);
            }
            current_bit_string.clear();
        }
        
        if current_bit_string.len() > 64 { // Huffman rzadko przekracza 64 bity
             panic!("Błąd: Nie znaleziono kodu w kontekście {:?}. String: {}", context, current_bit_string);
        }
    } else {
        break;
    }
}
    result
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Użycie: {} <input.huff> <output>", args[0]);
        return;
    }

    let content = fs::read(&args[1]).expect("Nie można otworzyć pliku wejściowego");
    let header = read_and_parse_header(&content).expect("Błąd parsowania nagłówka");

    let decoded = decode_data(
        &content[header.data_start_offset..],
        &header.markov_tables,
        header.order,
        header.original_len,
    );

    fs::write(&args[2], &decoded).expect("Błąd zapisu pliku wyjściowego");
    println!("✅ Zdekodowano {} bajtów.", decoded.len());
}
