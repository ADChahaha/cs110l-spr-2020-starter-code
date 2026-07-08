use std::env;
use std::fs::File;
use std::io::BufRead;
use std::{io, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    // Your code here :)
    let file = File::open(filename).unwrap();
    let mut reader = io::BufReader::new(file);
    let mut char_count: usize = 0;
    let mut word_count: usize = 0;
    let mut line_count: usize = 0;

    loop {
        let mut line = String::new();
        let line_len = reader.read_line(&mut line).unwrap();
        if line_len == 0 {
            break;
        }
        char_count += line_len;
        word_count += line.split_whitespace().count();
        line_count += 1;
    }

    println!("\t{}\t{}\t{} {}", line_count, word_count, char_count, filename);
}
