use sensitive_rs::Filter;

fn main() {
    let mut filter = Filter::new();
    filter.add_words(&["赌博", "色情", "诈骗"]);

    // Find the first sensitive word
    let (found, word) = filter.find_in("含有赌博内容");
    println!("Found: {found}, Word: {word}");

    // Find all sensitive words
    let words = filter.find_all("含有赌博和色情内容");
    println!("All words: {words:?}");

    // Replace sensitive words with a mask character
    let result = filter.replace("含有赌博内容", '*');
    println!("Replaced: {result}");

    // Filter sensitive words out entirely
    let result = filter.filter("含有赌博内容");
    println!("Filtered: {result}");
}
