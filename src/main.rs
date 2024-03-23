use anyhow::anyhow;
use clap::Parser as ClapParser;
use std::{collections::{HashMap, HashSet}, fs, path::PathBuf};
use nom::{branch::alt, bytes::complete::{tag, take_until, take_while_m_n}, character::complete::{alpha1, alphanumeric1, anychar, multispace0, space0}, combinator::{opt, recognize, rest}, multi::{many0, many1, many_till}, sequence::{delimited, tuple}, IResult};
use nom_locate::LocatedSpan;

////////////////////////////////////////////////////////////////////////////////
// Stucts and types
type Span<'a> = LocatedSpan<&'a str>;

#[derive(Debug, ClapParser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// HTML template that the Markdowns will use.
    #[arg(short, long, value_name = "FILE")]
    template: PathBuf,

    // num_args is required so that we don't have to specify the option before
    // each file...
    // `-m file.md file2.md`    rather than    `-m file.md -m file2.md`
    /// The directory or list of Markdown files.
    #[arg(short, long, value_name = "FILE", num_args = 1..)]
    markdowns: Vec<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub struct Meta {
    pub key: String,
    pub value: String,
}

// A position for a Cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Marker {
    line: u32,
    offset: usize,
}

impl Default for Marker {
    fn default() -> Self {
        Self {
            line: 1,
            offset: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Selection {
    start: Marker,
    end: Marker,
}

impl Selection {
    fn new(input: Span) -> Self {
        Self {
            start: Marker {
                line: input.location_line(),
                offset: input.location_offset(),
            },
            end: Marker {
                line: input.location_line(),
                offset: (input.location_offset() + input.fragment().len())
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Placeholder<'a> {
    span: Span<'a>,
    selection: Selection,
}

impl<'a> Placeholder<'a> {
    fn new(span: Span<'a>) -> Self {
        Self {
            span: span,
            selection: Selection::new(span),
        }
    }
}

impl<'a> Default for Placeholder<'a> {
    fn default() -> Self {
        Self {
            span: Span::new(""),
            selection: Selection::default(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Parsers
fn parse_meta_values(input: Span) -> IResult<Span, Meta> {
    let (input, key) = recognize(tuple((
        multispace0,
        alpha1,
        opt(many0(alt((alphanumeric1, tag("_")))))
    )))(input)?;

    let (input, _) = tuple((multispace0, tag("="), multispace0))(input)?;

    let (input, value) = alt((take_until("\n"), rest))(input)?;

    Ok((input, Meta { key: key.trim().to_string(), value: value.trim().to_string() }))
}

pub fn parse_meta_section(input: Span) -> IResult<Span, Option<Vec<Meta>>> {
    opt(delimited(
        tuple((multispace0, alt((tag(":meta"), tag("<meta>"))))),
        many1(parse_meta_values),
        tuple((multispace0, alt((tag(":meta"), tag("</meta>"))))),
    ))(input)
}

pub fn parse_title(input: Span) -> IResult<Span, Span> {
    let (input, _) = tuple((multispace0, tag("#"), space0))(input)?;
    let (input, title) = alt((take_until("\r\n"), take_until("\n")))(input)?;

    Ok((input.to_owned(), title.to_owned()))
}

fn is_variable(input: char) -> bool {
    vec!['a'..='z', 'A'..='Z', '0'..='9'].into_iter().flatten().find(|c| c == &input).is_some()
}

fn parse_variable(input: Span) -> IResult<Span, Span> {
    let (input, _) = tag("£")(input)?;
    let (input, variable) = recognize(tuple((
        take_while_m_n(1, 1, is_variable),
        many1(alt((alphanumeric1, tag("-"), tag("_")))),
    )))(input)?;

    Ok((input, variable))
}

fn parse_placeholder(input: Span) -> IResult<Span, Span> {
    recognize(tuple((
        tuple((tag("{{"), multispace0)),
        parse_variable,
        tuple((multispace0, tag("}}"))),
    )))(input)
}

fn take_till_placeholder(input: Span) -> IResult<Span, Span> {
    let (input, (_, placeholder)) = many_till(anychar, recognize(delimited(
        tuple((tag("{{"), multispace0)),
        parse_variable,
        tuple((multispace0, tag("}}"))),
    )))(input)?;

    Ok((input, placeholder))
}

/// This will consume the entire string!
fn parse_placeholder_locations(input: Span) -> Result<Vec<(String, Placeholder)>, anyhow::Error> {
    let mut old_input = input;
    let default_span = Span::new("");
    let mut placeholder = Span::new("start");
    let mut placeholders: Vec<(String, Placeholder)> = Vec::new();

    while placeholder != default_span {
        let (new_input, new_placeholder) = take_till_placeholder(old_input).unwrap_or((default_span, default_span));

        // Do another check because of the unwrap_or.
        if new_placeholder != default_span {
            placeholders.push(
                parse_placeholder(new_placeholder)
                .and_then(|(_, key)| {
                    Ok(key.replace("{", "").replace("£", "").replace("}", "").trim().to_string())
                })
                .and_then(|key| {
                    Ok((key, Placeholder::new(new_placeholder)))
                })
                .expect("variable to be extracted and added to placeholders")
            );
        }

        old_input = new_input;
        placeholder = new_placeholder;
    }

    // Sort in reverse so that when we replace each placeholder, the offsets do
    // not affect offsets after this point.
    placeholders.sort_by(|a, b| b.1.span.location_offset().cmp(&a.1.span.location_offset()));

    Ok(placeholders)
}

////////////////////////////////////////////////////////////////////////////////
// Functions

/// Replaces a substring in the original string with a replacement string.
///
/// # Arguments
///
/// * `original` - The original string.
/// * `start` - The start position of the substring in the original string.
/// * `end` - The end position of the substring in the original string.
/// * `replacement` - The string to replace the substring.
///
/// # Returns
///
/// * A new string with the replacement in place of the original substring.
///
/// # Example
///
/// ```
/// let original = "Hello, world!";
/// let start = 7;
/// let end = 12;
/// let replacement = "Rust";
/// let result = replace_substring(original, start, end, replacement);
/// println!("{}", result);  // Prints: "Hello, Rust!"
/// ```
fn replace_substring(original: &str, start: usize, end: usize, replacement: &str) -> String {
    let mut result = String::new();
    result.push_str(&original[..start]);
    result.push_str(replacement);
    result.push_str(&original[end..]);
    result
}

/// Return a replacement string for a given key.
///
/// # Arguments
///
/// * `markdown` - The markdown string.
/// * `meta_values` - A kay-value pair of meta values.
/// * `key` - The key to look up the replacement for.
///
/// # Returns
///
/// * The replacement string for the given key if found.
/// * An empty string if no replacement is found.
///
/// # Example
/// ```
/// let markdown = Span::new("# Markdown title\nContent paragraph");
/// let meta_values = parse_meta_section(markdown).unwrap_or((markdown, Some(vec![])));
/// let key = "title";
/// let replacements = get_replacement(markdown, &meta_values, &key);
/// ```
fn get_replacement(markdown: Span, meta_values: &HashMap<String, String>, key: &str) -> String {
    match key {
        "title" => match meta_values.contains_key("title") {
            true => meta_values.get("title").unwrap().to_owned(),
            false => match parse_title(markdown) {
                Ok((_, title)) => title.to_string(),
                _ => "".to_owned(),
            },
        },
        "content" => markdown.trim().to_string(),
        key if meta_values.contains_key(key) => {
            meta_values.get(key).unwrap().to_string()
        },
        _ => "".to_string(),
    }
}




fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let template = cli.template;

    // Check that the actual template exists.
    if !template.try_exists().map_err(|_| anyhow!("The template could not be found."))? {
       Err(anyhow!("The template file does not exist."))?;
    };
    let template = std::fs::read_to_string(&template)?;


    // Get only existing markdowns.
    let markdown_urls: Vec<PathBuf> = cli.markdowns
        .into_iter()
        .filter(|file| file.exists() && file.extension().unwrap_or_default() == "md" )
        .collect();
    let markdowns: Vec<String> = markdown_urls
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect();


    // Get the keys first because a HashMap is not ordered.
    let placeholders = parse_placeholder_locations(Span::new(&template))?;
    let placeholder_keys: Vec<String> = placeholders.iter().map(|(key, _)| key.to_string()).collect();
    let placeholders: HashMap<String, Placeholder> = placeholders.into_iter().collect();

    if !placeholders.contains_key("title") && !placeholders.contains_key("content") {
        Err(anyhow!("Template must define 'title' and 'content' placeholders"))?;
    }

    let html_docs: Result<Vec<String>, anyhow::Error> = markdowns
    .into_iter()
    .map(|markdown| {
        let markdown = Span::new(&markdown);
        let mut html_doc = template.clone();

        // Attempt to retrieve a key-value of meta values, and return the
        // leftover markdown.
        // This function returns an Option, because nom would fail if there is
        // no meta, so even though we unwrap_or, we still need to unwrap.
        let (markdown, meta_values) = parse_meta_section(markdown).unwrap_or((markdown, Some(vec![])));
        let meta_values = meta_values.unwrap_or_default();
        let meta_values: HashMap<String, String> = meta_values.into_iter().map(|m| (m.key, m.value)).collect();

        // REVIEW This is a really lame system, but it checks to ensure that the
        // markdown and template have the same keys.
        let mut markdown_keys: HashSet<&String> = meta_values.keys().collect();
        let content_string = "title".to_string();
        markdown_keys.insert(&content_string);
        let content_string = "content".to_string();
        markdown_keys.insert(&content_string);
        let placeholder_keys_set: HashSet<&String> = placeholder_keys.iter().collect();

        if placeholder_keys_set.difference(&markdown_keys).count() > 0 {
            return Err(anyhow!("Template must define all meta values. Missing: {:?}", placeholder_keys_set.difference(&markdown_keys)));
        }
        drop(markdown_keys);
        drop(content_string);
        drop(placeholder_keys_set);

        for key in &placeholder_keys {
            let placeholder = placeholders.get(key).unwrap();
            let replacements = get_replacement(markdown, &meta_values, &key);

            // FIXME The meta values are creating <p> tags which isn't ideal.
            // We don't want to apply Markdown to the
            let replacements = if key == "content" {
                markdown::to_html(&replacements)
            } else {
                replacements
            };

            html_doc = replace_substring(&html_doc, placeholder.selection.start.offset, placeholder.selection.end.offset, &replacements);
        }

        Ok(html_doc)
    })
    .collect();

    let html_docs = html_docs?;
    let html_docs = markdown_urls.into_iter().zip(html_docs);

    for (url, html_doc) in html_docs {
        let output_path = url.with_extension("html");
        fs::write(output_path, html_doc)?;
    }

    Ok(())
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_find_variable() {
        let input = Span::new("£content }}");
        let (input, variable) = parse_variable(input).unwrap();

        assert_eq!(variable.fragment(), &"content");
        assert_eq!(input.fragment(), &" }}");
    }

    #[test]
    fn can_find_variable_with_underscore() {
        let input = Span::new("£publish_date }}");
        let (input, variable) = parse_variable(input).unwrap();

        assert_eq!(variable.fragment(), &"publish_date");
        assert_eq!(input.fragment(), &" }}");
    }

    #[test]
    fn can_find_placeholder() {
        let input = Span::new("{{ £content }}\nTemplate content");
        let parsed_placeholder = parse_placeholder(input);

        assert!(parsed_placeholder.is_ok());

        let (input, placeholder) = parsed_placeholder.unwrap();
        assert_eq!(placeholder.fragment(), &"{{ £content }}");
        assert_eq!(input.fragment(), &"\nTemplate content");
    }

    #[test]
    fn can_find_md_title() {
        let markdown = Span::new("# My Title\nMy content");
        let parsed_title = parse_title(markdown);

        assert!(parsed_title.is_ok());

        let (input, title) = parsed_title.unwrap();
        assert_eq!(title.fragment(), &"My Title");
        assert_eq!(input.fragment(), &"\nMy content");
    }

    #[test]
    fn can_parse_meta_value() {
        let input = Span::new("title = My Title");
        let (_, meta) = parse_meta_values(input).expect("to parse meta key-value");
        assert_eq!(meta, Meta { key: "title".to_string(), value: "My Title".to_string() });
    }

    #[test]
    fn can_parse_meta_value_with_underscore() {
        let input = Span::new("publish_date = 2024-01-01");
        dbg!(input);
        let (_, meta) = parse_meta_values(input).expect("to parse meta key-value");
        assert_eq!(meta, Meta { key: "publish_date".to_string(), value: "2024-01-01".to_string() });
    }

    #[test]
    fn can_parse_metadata_colon() {
        let input = Span::new(":meta\ntitle = Meta title\nauthor = John Doe\n:meta\n# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

        assert!(meta.is_some());
        assert_eq!(meta.unwrap(), vec![
            Meta { key: "title".to_string(), value: "Meta title".to_string() },
            Meta { key: "author".to_string(), value: "John Doe".to_string() },
        ]);

        assert_eq!(input.fragment(), &"\n# Markdown title\nThis is my content");
    }

    #[test]
    fn can_parse_metadata_tag() {
        let input = Span::new("<meta>\ntitle = Meta title\nauthor = John Doe\n</meta>\n# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

        assert!(meta.is_some());
        assert_eq!(meta.unwrap(), vec![
            Meta { key: "title".to_string(), value: "Meta title".to_string() },
            Meta { key: "author".to_string(), value: "John Doe".to_string() },
        ]);

        assert_eq!(input.fragment(), &"\n# Markdown title\nThis is my content");
    }

    #[test]
    fn can_parse_when_no_meta_section() {
        let input = Span::new("# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

        assert!(meta.is_none());
        assert_eq!(input.fragment(), &"# Markdown title\nThis is my content");
    }

    #[test]
    fn can_replace_placeholder_from_meta() {
        let input = Span::new("<meta>\ntitle = Meta title\nauthor = John Doe\n</meta>\n# Markdown title\nThis is my content");
        let template = Span::new("<html>\n<head>\n<title>{{ £title }}</title>\n</head>\n<body>\n<section>{{ £content }}</section>\n<p>By {{ £author }}</p>\n</body>\n</html>");

        let placeholders = parse_placeholder_locations(template).expect("to parse placeholders");
        let placeholder_keys: Vec<String> = placeholders.iter().map(|(key, _)| key.to_string()).collect();
        let placeholders: HashMap<String, Placeholder> = placeholders.into_iter().collect();

        assert_eq!(placeholders.get("title").unwrap().selection, Selection {
            start: Marker { line: 3, offset: 21 },
            end: Marker { line: 3, offset: 34 },
        });

        assert_eq!(placeholders.get("content").unwrap().selection, Selection {
            start: Marker { line: 6, offset: 67 },
            end: Marker { line: 6, offset: 82 },
        });

        assert_eq!(placeholders.get("author").unwrap().selection, Selection {
            start: Marker { line: 7, offset: 99 },
            end: Marker { line: 7, offset: 113 },
        });

        let (markdown, meta_values) = parse_meta_section(input).expect("to parse the meta values");

        assert!(meta_values.is_some());
        let meta_values = meta_values.unwrap_or_default();
        assert_eq!(meta_values, vec![
            Meta { key: "title".to_string(), value: "Meta title".to_string() },
            Meta { key: "author".to_string(), value: "John Doe".to_string() },
        ]);

        let meta_values: HashMap<String, String> = meta_values.into_iter().map(|m| (m.key, m.value)).collect();

        let mut html_doc = template.to_string();
        for key in placeholder_keys {
            let placeholder = placeholders.get(&key).unwrap();
            let replacements = get_replacement(markdown, &meta_values, &key);

            html_doc = replace_substring(&html_doc, placeholder.selection.start.offset, placeholder.selection.end.offset, &replacements);
        }

        assert_eq!(html_doc, "<html>\n<head>\n<title>Meta title</title>\n</head>\n<body>\n<section># Markdown title\nThis is my content</section>\n<p>By John Doe</p>\n</body>\n</html>");
    }

    #[test]
    fn can_parse_empty_string_when_no_value_found_for_placeholder() {
        let input = Span::new("# Markdown title\nThis is my content");
        let template = Span::new("<html>\n<head>\n<title>{{ £title }}</title>\n</head>\n<body>\n<section>{{ £content }}</section>\n<p>By {{ £author }}</p>\n</body>\n</html>");

        let placeholders = parse_placeholder_locations(template).expect("to parse placeholders");
        let placeholder_keys: Vec<String> = placeholders.iter().map(|(key, _)| key.to_string()).collect();
        let placeholders: HashMap<String, Placeholder> = placeholders.into_iter().collect();

        assert_eq!(placeholders.get("title").unwrap().selection, Selection {
            start: Marker { line: 3, offset: 21 },
            end: Marker { line: 3, offset: 34 },
        });

        assert_eq!(placeholders.get("content").unwrap().selection, Selection {
            start: Marker { line: 6, offset: 67 },
            end: Marker { line: 6, offset: 82 },
        });

        assert_eq!(placeholders.get("author").unwrap().selection, Selection {
            start: Marker { line: 7, offset: 99 },
            end: Marker { line: 7, offset: 113 },
        });

        let (markdown, meta_values) = parse_meta_section(input).expect("to parse the meta values");

        assert!(meta_values.is_none());
        let meta_values = meta_values.unwrap_or_default();
        let meta_values: HashMap<String, String> = meta_values.into_iter().map(|m| (m.key, m.value)).collect();

        let mut html_doc = template.to_string();
        for key in placeholder_keys {
            let placeholder = placeholders.get(&key).unwrap();
            let replacements = get_replacement(markdown, &meta_values, &key);

            html_doc = replace_substring(&html_doc, placeholder.selection.start.offset, placeholder.selection.end.offset, &replacements);
        }

        assert_eq!(html_doc, "<html>\n<head>\n<title>Markdown title</title>\n</head>\n<body>\n<section># Markdown title\nThis is my content</section>\n<p>By </p>\n</body>\n</html>");
    }
}
