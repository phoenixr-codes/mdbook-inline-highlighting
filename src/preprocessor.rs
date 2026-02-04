use mdbook_preprocessor::book::{Book, BookItem, Chapter};
use mdbook_preprocessor::errors::Result;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};
use pulldown_cmark::{Event, Options, Parser};
use pulldown_cmark_to_cmark::cmark;

use crate::config::Configuration;

const ESCAPE_CHAR: char = '\\';
const LANG_SPEC_START: char = '[';
const LANG_SPEC_END: char = ']';

#[derive(Default)]
pub(crate) struct InlineHighlighterPreprocessor;

impl Preprocessor for InlineHighlighterPreprocessor {
    fn name(&self) -> &str {
        "inline-highlighting"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        let config = Configuration::from_mdbook_config(&ctx.config);
        let default_language = config.default_language;

        let smart_quotes = ctx
            .config
            .get::<bool>("output.html.smart-punctuation")
            .ok()
            .flatten()
            .unwrap_or(false);

        book.for_each_mut(|item: &mut BookItem| {
            if let BookItem::Chapter(chapter) = item {
                let mut buf = String::new();

                let parser = new_cmark_parser(&chapter.content, smart_quotes);
                let mut events = vec![];
                for event in parser {
                    events.push(if let Event::Code(code) = event {
                        let (c, is_html) =
                            parse_inline_code(code.as_ref(), default_language.as_deref(), chapter);
                        if is_html {
                            Event::Html(c.into())
                        } else {
                            Event::Code(c.into())
                        }
                    } else {
                        event
                    });
                }
                match cmark(events.iter(), &mut buf).map(|_| buf) {
                    Ok(result) => chapter.content = result,
                    Err(error) => {
                        log::error!("Markdown serialization failed: {}", error);
                    }
                };
            };
        });
        Ok(book)
    }

    fn supports_renderer(&self, renderer: &str) -> Result<bool> {
        Ok(renderer == "html")
    }
}

fn new_cmark_parser<'a>(text: &'a str, smart_punctuation: bool) -> Parser<'a> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    if smart_punctuation {
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
    }
    Parser::new_ext(text, options)
}

/// Returns a tuple with the first item being the new content and the second item
/// a boolean whether it is an HTML node.
fn parse_inline_code(
    code: &str,
    default_language: Option<&str>,
    chapter: &Chapter,
) -> (String, bool) {
    let mut chars = code.chars();
    match chars.next() {
        Some(LANG_SPEC_START) => {}
        Some(ch) => {
            let result: &str = if ch == ESCAPE_CHAR {
                chars.as_str()
            } else {
                code
            };

            return if let Some(l) = default_language {
                (inline_with_highlighting(result, l), true)
            } else {
                (result.to_string(), false)
            };
        }
        None => return (String::new(), false),
    };
    let mut lang = String::new();
    loop {
        let maybe_ch = chars.next();
        match maybe_ch {
            Some(LANG_SPEC_END) => break,
            Some(ch) => lang.push(ch),
            None => {
                log::error!(
                    "missing closing character `{}` in chapter `{}`",
                    LANG_SPEC_END,
                    chapter
                );
                return if let Some(l) = default_language {
                    (inline_with_highlighting(code, l), true)
                } else {
                    (code.into(), false)
                };
            }
        };
    }
    let language: Option<&str> = if lang == "none" {
        default_language
    } else {
        Some(&lang)
    };
    if chars.next().is_none_or(|ch| ch != ' ') {
        log::error!(
            "missing space after language identifier in chapter `{}`",
            chapter
        );
        return if let Some(l) = default_language {
            (inline_with_highlighting(code, l), true)
        } else {
            (code.into(), false)
        };
    };
    let actual_code = chars.as_str();
    match language {
        Some(l) => (inline_with_highlighting(actual_code, l), true),
        None => (actual_code.to_string(), false),
    }
}

fn inline_with_highlighting(code: &str, language: &str) -> String {
    format!("<code class=\"hljs language-{}\">{}</code>", language, code)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn html_with_language() {
        assert_eq!(
            "<code class=\"hljs language-javascript\">Hello</code>",
            inline_with_highlighting("Hello", "javascript"),
        );
    }

    #[test]
    fn invalid_inline() {
        assert_eq!(
            ("[forgot-to-close oops".to_string(), false),
            parse_inline_code("[forgot-to-close oops", None, &Chapter::default())
        );
        assert_eq!(
            (
                "<code class=\"hljs language-javascript\">[forgot-to-close oops</code>".to_string(),
                true
            ),
            parse_inline_code(
                "[forgot-to-close oops",
                Some("javascript"),
                &Chapter::default()
            )
        );
        assert_eq!(
            ("[js]var missingSpace;".to_string(), false),
            parse_inline_code("[js]var missingSpace;", None, &Chapter::default()),
        );
        assert_eq!(
            (
                "<code class=\"hljs language-typescript\">[js]var missingSpace;</code>".to_string(),
                true
            ),
            parse_inline_code(
                "[js]var missingSpace;",
                Some("typescript"),
                &Chapter::default()
            )
        )
    }

    #[test]
    fn escaped_inline() {
        assert_eq!(
            ("[python] x = 1".to_string(), false),
            parse_inline_code("\\[python] x = 1", None, &Chapter::default())
        );
        assert_eq!(
            (
                "<code class=\"hljs language-python\">[Hello</code>".to_string(),
                true
            ),
            parse_inline_code("\\[Hello", Some("python"), &Chapter::default())
        );
    }

    #[test]
    fn markdown_without_default_without_language() {
        let expect = String::from("Hello");
        assert_eq!(
            (expect.clone(), false),
            parse_inline_code("[none] Hello", None, &Chapter::default()),
        );
        assert_eq!(
            (expect.clone(), false),
            parse_inline_code("Hello", None, &Chapter::default()),
        )
    }

    #[test]
    fn markdown_with_default_without_language() {
        let expect = String::from("<code class=\"hljs language-javascript\">Hello</code>");
        assert_eq!(
            (expect.clone(), true),
            parse_inline_code("[none] Hello", Some("javascript"), &Chapter::default()),
        );
        assert_eq!(
            (expect.clone(), true),
            parse_inline_code("Hello", Some("javascript"), &Chapter::default()),
        );
    }

    #[test]
    fn markdown_without_default_with_language() {
        assert_eq!(
            (
                "<code class=\"hljs language-javascript\">Hello</code>".to_string(),
                true
            ),
            parse_inline_code("[javascript] Hello", None, &Chapter::default()),
        )
    }

    #[test]
    fn markdown_with_default_with_language() {
        assert_eq!(
            (
                "<code class=\"hljs language-javascript\">Hello</code>".to_string(),
                true
            ),
            parse_inline_code("[javascript] Hello", Some("python"), &Chapter::default()),
        )
    }
}
