use std::{collections::HashMap, fs};
use nom::{branch::alt, bytes::complete::{tag, take_until, take_while_m_n}, character::complete::{alpha1, alphanumeric0, alphanumeric1, anychar, multispace0, space0}, combinator::{opt, recognize, rest}, multi::{many0, many1, many_till}, sequence::{delimited, tuple}, IResult};
use nom_locate::LocatedSpan;

////////////////////////////////////////////////////////////////////////////////
// Stucts and types
type Span<'a> = LocatedSpan<&'a str>;

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
        opt(many0(alt((alphanumeric0, tag("_")))))
    )))(input)?;

    let (input, _) = tuple((multispace0, tag("="), multispace0))(input)?;

    let (input, value) = alt((take_until("\n"), rest))(input)?;

    Ok((input, Meta { key: key.trim().to_string(), value: value.trim().to_string() }))
}

pub fn parse_meta_section(input: Span) -> IResult<Span, Vec<Meta>> {
    delimited(
        tuple((multispace0, alt((tag(":meta"), tag("<meta>"))))),
        many1(parse_meta_values),
        tuple((multispace0, alt((tag(":meta"), tag("</meta>"))))),
    )(input)
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
fn parse_placeholder_locations(input: Span) -> Result<HashMap<String, Placeholder>, anyhow::Error> {
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
    let placeholders: HashMap<String, Placeholder> = placeholders.into_iter().collect();

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



fn main() -> Result<(), anyhow::Error> {
    let template = fs::read_to_string("target/debug/template.html")?;
    let markdowns = vec![
        fs::read_to_string("target/debug/a.md")?,
        fs::read_to_string("target/debug/b.md")?,
        fs::read_to_string("target/debug/c.md")?,
    ];

    let placeholders = parse_placeholder_locations(Span::new(&template))?;

    let html_docs: Vec<String> = markdowns
    .into_iter()
    .map(|markdown| {
        let markdown = Span::new(&markdown);
        let mut new_markdown = template.clone();

        let (markdown, meta_values) = parse_meta_section(markdown).unwrap_or((markdown, vec![]));
        let meta_values: HashMap<String, String> = meta_values.into_iter().map(|m| (m.key, m.value)).collect();

        // First check if there is a <meta> title.
        // If not, then see if we can parse an <h1> tag, and use that.
        let title = match meta_values.contains_key("title") {
            true => meta_values.get("title").unwrap().to_owned(),
            false => match parse_title(markdown) {
                Ok((_, title)) => title.to_string(),
                _ => "".to_owned(),
            },
        };

        for (key, placeholder) in &placeholders {
            // Title is special because it might appear in the meta data.
            let replacements = match key.as_str() {
                "title" => title.trim().to_string(),
                _ => markdown.trim().to_string(),
            };

            new_markdown = replace_substring(
                &new_markdown,
                placeholder.selection.start.offset,
                placeholder.selection.end.offset,
                &replacements,
            );
        }

        new_markdown
    })
    .collect();

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
    fn can_parse_metadata_colon() {
        let input = Span::new(":meta\ntitle = Meta title\nauthor = John Doe\n:meta\n# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta");

        assert_eq!(vec![
            Meta { key: "title".to_string(), value: "Meta title".to_string() },
            Meta { key: "author".to_string(), value: "John Doe".to_string() },
        ], meta);

        assert_eq!(input.fragment(), &"\n# Markdown title\nThis is my content");
    }

    #[test]
    fn can_parse_metadata_tag() {
        let input = Span::new("<meta>\ntitle = Meta title\nauthor = John Doe\n</meta>\n# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta");

        assert_eq!(vec![
            Meta { key: "title".to_string(), value: "Meta title".to_string() },
            Meta { key: "author".to_string(), value: "John Doe".to_string() },
        ], meta);

        assert_eq!(input.fragment(), &"\n# Markdown title\nThis is my content");
    }
}
