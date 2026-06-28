// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn print_current_word_chars(current_word_chars: &Vec<char>) {
    let current_string: String = current_word_chars.iter().collect();
    println!("The word so far is {}", current_string);
}
fn print_guessed_chars(guessed_chars: &Vec<char>) {
    let guessed_string: String = guessed_chars.iter().collect();
    println!("You have guessed the following letters: {}", guessed_string);
}

fn find_all_index_of_element(v: &Vec<char>, c: char) -> Vec<usize> {
    let mut indexs: Vec<usize> = Vec::new();
    for i in 0..v.len() {
        if v[i] == c {
            indexs.push(i);
        }
    }
    indexs
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    // println!("random word: {}", secret_word);

    // Your code here! :)

    println!("Welcome to CS110L Hangman!");
    let mut remaining_guess = NUM_INCORRECT_GUESSES;
    let mut current_word_chars = vec!['-'; secret_word_chars.len()];
    let mut guessed_chars: Vec<char> = Vec::new();
    while remaining_guess > 0 && current_word_chars.contains(&'-') {
        print_current_word_chars(&current_word_chars);
        print_guessed_chars(&guessed_chars);
        println!("You have {} guesses left", remaining_guess);
        print!("Please guess a letter: ");
        io::stdout().flush().expect("Error flushing stdout.");

        // 获取输入
        let mut guess = String::new();
        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line.");
        let chars: Vec<char> = guess.trim().chars().collect();
        // 检验是否为一个字母
        if chars.len() != 1 || !chars[0].is_alphabetic() || !chars[0].is_ascii_lowercase() {
            println!("input must be one lower character");
            continue;
        }
        let char = chars[0];
        // 加入猜过的字母
        guessed_chars.push(char);
        //找到所有满足的下标
        let indexs = find_all_index_of_element(&secret_word_chars, char);
        //如果不在
        if indexs.is_empty() {
            remaining_guess -= 1;
            println!("Sorry, that letter is not in the word");
        } else {
            //如果在
            // current_word_chars对应的位置更新
            for index in indexs {
                current_word_chars[index] = char;
            }
        }
        //空行
        println!();
    }
    if remaining_guess == 0 {
        println!("Sorry, you ran out of guesses!");
    }
    else{
        println!("You Win!")
    }
}
