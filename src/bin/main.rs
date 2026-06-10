use clap::{Parser, Subcommand, ValueEnum};
use sensitive_rs::{Filter, MatchAlgorithm};
use serde::Serialize;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::process;

/// A CLI tool for sensitive word detection and filtering
#[derive(Parser)]
#[command(name = "sensitive-rs", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Custom dictionary file path
    #[arg(long, global = true)]
    dict: Option<String>,

    /// Use extended dictionary (dict/dict-all.txt)
    #[arg(long, global = true)]
    dict_all: bool,

    /// Force matching algorithm
    #[arg(long, value_enum, global = true)]
    algorithm: Option<AlgorithmArg>,

    /// Enable pinyin and shape variant detection
    #[arg(long, global = true)]
    variant: bool,

    /// Custom noise removal regex pattern
    #[arg(long, global = true)]
    noise_pattern: Option<String>,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    /// Force colored output (auto-detected by default)
    #[arg(long, global = true, num_args = 0..=1, default_missing_value = "true")]
    color: Option<bool>,
}

#[derive(Subcommand)]
enum Commands {
    /// Find all sensitive words in text
    Check {
        /// Text to check (reads from stdin if omitted)
        text: Option<String>,

        /// Read from file(s)
        #[arg(short, long)]
        file: Vec<String>,
    },

    /// Exit 0 if text is clean, exit 1 if sensitive words found
    Validate {
        /// Text to validate (reads from stdin if omitted)
        text: Option<String>,

        /// Read from file(s)
        #[arg(short, long)]
        file: Vec<String>,
    },

    /// Replace sensitive words with a character
    Replace {
        /// Replacement character
        replacement: char,

        /// Text to process (reads from stdin if omitted)
        text: Option<String>,

        /// Read from file(s)
        #[arg(short, long)]
        file: Vec<String>,
    },

    /// Remove sensitive words entirely
    Filter {
        /// Text to filter (reads from stdin if omitted)
        text: Option<String>,

        /// Read from file(s)
        #[arg(short, long)]
        file: Vec<String>,
    },
}

#[derive(Clone, ValueEnum)]
enum AlgorithmArg {
    AhoCorasick,
    WuManber,
    Regex,
}

impl From<AlgorithmArg> for MatchAlgorithm {
    fn from(arg: AlgorithmArg) -> Self {
        match arg {
            AlgorithmArg::AhoCorasick => MatchAlgorithm::AhoCorasick,
            AlgorithmArg::WuManber => MatchAlgorithm::WuManber,
            AlgorithmArg::Regex => MatchAlgorithm::Regex,
        }
    }
}

#[derive(Serialize)]
struct CheckResult {
    found: usize,
    words: Vec<WordMatch>,
}

#[derive(Serialize)]
struct WordMatch {
    word: String,
}

#[derive(Serialize)]
struct ValidateResult {
    clean: bool,
    found: usize,
    word: Option<String>,
}

#[derive(Serialize)]
struct ProcessResult {
    output: String,
}

fn build_filter(cli: &Cli) -> Filter {
    let mut filter = if let Some(algo) = &cli.algorithm {
        Filter::with_algorithm(algo.clone().into())
    } else {
        Filter::new()
    };

    if let Some(pattern) = &cli.noise_pattern {
        filter.update_noise_pattern(pattern);
    }

    if cli.dict_all {
        if let Err(e) = filter.load_word_dict("dict/dict-all.txt") {
            eprintln!("Error: failed to load extended dictionary: {e}");
            process::exit(1);
        }
    } else if let Some(dict_path) = &cli.dict {
        if let Err(e) = filter.load_word_dict(dict_path) {
            eprintln!("Error: failed to load dictionary from '{}': {e}", dict_path);
            process::exit(1);
        }
    } else if let Err(e) = filter.load_word_dict("dict/dict.txt") {
        eprintln!("Error: failed to load default dictionary: {e}");
        process::exit(1);
    }

    filter
}

fn resolve_texts<'a>(text: &'a Option<String>, files: &'a [String]) -> Vec<(String, Option<&'a str>)> {
    let mut texts = Vec::new();

    if let Some(t) = text {
        texts.push((t.clone(), None));
    }

    for path in files {
        match fs::read_to_string(path) {
            Ok(content) => texts.push((content, Some(path.as_str()))),
            Err(e) => {
                eprintln!("Error: failed to read file '{path}': {e}");
                process::exit(1);
            }
        }
    }

    if texts.is_empty() {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("Error: failed to read stdin: {e}");
            process::exit(1);
        }
        if buf.is_empty() {
            eprintln!("Error: no input provided. Pass text as argument, use --file, or pipe via stdin.");
            process::exit(1);
        }
        texts.push((buf, None));
    }

    texts
}

fn use_color(cli: &Cli) -> bool {
    match cli.color {
        Some(force) => force,
        None => io::stderr().is_terminal(),
    }
}

fn colored(text: &str, code: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn cmd_check(cli: &Cli, filter: &Filter, texts: Vec<(String, Option<&str>)>) {
    let color = use_color(cli);
    let mut all_results: Vec<serde_json::Value> = Vec::new();

    for (text, source) in &texts {
        let words = filter.find_all(text);

        if cli.json {
            let result = CheckResult {
                found: words.len(),
                words: words.iter().map(|w| WordMatch { word: w.clone() }).collect(),
            };
            all_results.push(serde_json::json!({
                "source": source,
                "result": result,
            }));
        } else if words.is_empty() {
            let label = source.map(|s| format!("[{s}] ")).unwrap_or_default();
            println!("{label}{}", colored("No sensitive words found.", "32", color));
        } else {
            let label = source.map(|s| format!("[{s}] ")).unwrap_or_default();
            println!(
                "{}Found {} sensitive word(s):",
                label,
                colored(&words.len().to_string(), "31", color),
            );
            for word in &words {
                println!("  {}", colored(word, "33", color));
            }
        }
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&all_results).unwrap());
    }
}

fn cmd_validate(cli: &Cli, filter: &Filter, texts: Vec<(String, Option<&str>)>) {
    let mut all_clean = true;
    let mut all_results: Vec<serde_json::Value> = Vec::new();

    for (text, source) in &texts {
        let (found, word) = filter.validate(text);

        if cli.json {
            let result = ValidateResult {
                clean: !found,
                found: if found { 1 } else { 0 },
                word: if found { Some(word) } else { None },
            };
            all_results.push(serde_json::json!({
                "source": source,
                "result": result,
            }));
        } else if found {
            all_clean = false;
            let label = source.map(|s| format!("[{s}] ")).unwrap_or_default();
            println!("{label}Sensitive word found: {word}");
        }
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&all_results).unwrap());
    }

    if !all_clean {
        process::exit(1);
    }
}

fn cmd_replace(cli: &Cli, filter: &Filter, replacement: char, texts: Vec<(String, Option<&str>)>) {
    let mut all_results: Vec<serde_json::Value> = Vec::new();

    for (text, source) in &texts {
        let output = filter.replace(text, replacement);

        if cli.json {
            let result = ProcessResult {
                output: output.clone(),
            };
            all_results.push(serde_json::json!({
                "source": source,
                "result": result,
            }));
        } else {
            println!("{output}");
        }
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&all_results).unwrap());
    }
}

fn cmd_filter(cli: &Cli, filter: &Filter, texts: Vec<(String, Option<&str>)>) {
    let mut all_results: Vec<serde_json::Value> = Vec::new();

    for (text, source) in &texts {
        let output = filter.filter(text);

        if cli.json {
            let result = ProcessResult {
                output: output.clone(),
            };
            all_results.push(serde_json::json!({
                "source": source,
                "result": result,
            }));
        } else {
            println!("{output}");
        }
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&all_results).unwrap());
    }
}

fn main() {
    let cli = Cli::parse();
    let filter = build_filter(&cli);

    match &cli.command {
        Commands::Check { text, file } => {
            let texts = resolve_texts(text, file);
            cmd_check(&cli, &filter, texts);
        }
        Commands::Validate { text, file } => {
            let texts = resolve_texts(text, file);
            cmd_validate(&cli, &filter, texts);
        }
        Commands::Replace {
            replacement,
            text,
            file,
        } => {
            let texts = resolve_texts(text, file);
            cmd_replace(&cli, &filter, *replacement, texts);
        }
        Commands::Filter { text, file } => {
            let texts = resolve_texts(text, file);
            cmd_filter(&cli, &filter, texts);
        }
    }
}
