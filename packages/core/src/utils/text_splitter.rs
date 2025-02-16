use text_splitter::{MarkdownSplitter, TextSplitter};

pub fn split_text(
    text: &str,
    splitter: Option<&TextSplitter<tokenizers::Tokenizer>>,
    md_splitter: Option<&MarkdownSplitter<tokenizers::Tokenizer>>,
) -> Vec<String> {
    if let Some(md_splitter) = md_splitter {
        md_splitter
            .chunks(text)
            .map(|item| item.to_string())
            .collect()
    } else if let Some(splitter) = splitter {
        splitter.chunks(text).map(|item| item.to_string()).collect()
    } else {
        println!("No splitter found");
        vec![]
    }
}
