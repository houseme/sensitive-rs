use sensitive_rs::Filter;

fn main() {
    let mut filter = Filter::new();
    filter.add_word("赌博");

    // Pinyin variant detection: "dubo" maps back to "赌博"
    let (found, word) = filter.find_in("含有 dubo 内容");
    println!("Pinyin variant: found={found}, word={word}");

    // Shape variant detection: "睹" is a shape variant of "赌"
    let (found, word) = filter.find_in("含有睹博内容");
    println!("Shape variant: found={found}, word={word}");
}
