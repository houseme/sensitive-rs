use sensitive_rs::Filter;
use std::io::Cursor;

fn main() {
    let mut filter = Filter::new();

    // Load a dictionary from an in-memory reader (also works with a file via
    // filter.load_word_dict("my_dict.txt")).
    let dict = "赌博\n色情\n诈骗";
    filter.load(Cursor::new(dict)).unwrap();

    let words = filter.find_all("含有赌博内容");
    println!("Found: {words:?}");
}
