type Document = Vec<String>;

fn new_document(words: Vec<String>) -> Document {
    words
}

fn add_word(this: &mut Document, word: String) {
    this.push(word);
}

fn get_words(this: &Document) -> &[String] {
    this.as_slice()
}

fn get_first(strs: &mut (String, String)) -> &mut String {
    &mut strs.0
}
fn get_second(strs: &mut (String, String)) -> &mut String {
    &mut strs.1
}
// fn transfer_string(strs: &mut (String, String)) {
//     let fst = get_first(strs);
//     let snd = get_second(strs);
//     fst.push_str(snd);
//     snd.clear();
// }

fn main() {
    let words = vec!["hello".to_string()];
    let d = new_document(words);

    let words_copy = get_words(&d).to_vec();
    let mut d2 = new_document(words_copy);
    add_word(&mut d2, "world".to_string());
    println!("{:?}", d2);
    println!("{:?}", d);

    // transfer_string(&mut ("hello".to_string(), "world".to_string()));
    // println!("{:?}", ("hello".to_string(), "world".to_string()));
}
