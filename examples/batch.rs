use sensitive_rs::Filter;

fn main() {
    let mut filter = Filter::new();
    filter.add_words(&["赌博", "色情", "诈骗"]);

    let texts = vec!["第一条含有赌博", "第二条含有色情", "第三条正常", "第四条含有诈骗"];

    let results = filter.find_all_batch(&texts);
    for (text, words) in texts.iter().zip(results.iter()) {
        println!("{text} => {words:?}");
    }
}
