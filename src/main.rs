use anyhow::anyhow;
use clap::Parser as ClapParser;
use std::{collections::HashMap, fs, path::PathBuf};
use nom::{branch::alt, bytes::complete::{tag, take_till, take_until, take_while, take_while_m_n}, character::complete::{alphanumeric1, anychar, multispace0, space0}, combinator::{opt, recognize, rest}, multi::{many0, many1, many_till}, sequence::{delimited, preceded, separated_pair, terminated, tuple}, IResult, Parser};
use nom_locate::LocatedSpan;

////////////////////////////////////////////////////////////////////////////////
// Stucts and types
type Span<'a> = LocatedSpan<&'a str>;

#[derive(Debug, ClapParser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// HTML template that the Markdowns will populate.
    #[arg(short, long, required = true, value_name = "FILE")]
    template: PathBuf,

    // num_args is required so that we don't have to specify the option before
    // each file...
    // `-m file.md file2.md`    rather than    `-m file.md -m file2.md`
    /// List of Markdown files ending in .md.
    #[arg(short, long, required = true, value_name = "FILES", num_args = 1..)]
    markdowns: Vec<PathBuf>,

    /// Output directory, defaults to the current directory.
    #[arg(short, long, value_name = "DIR")]
    output_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Directive {
    Date(String),
    Uppercase,
    Lowercase,
    Markdown,
}

#[derive(Debug, PartialEq)]
pub struct Meta {
    pub key: String,
    pub value: String,
    pub directives: Vec<Directive>,
}

impl Meta {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.trim().to_string(),
            value: value.trim().to_string(),
            directives: Vec::new(),
        }
    }
}

// A position for a Cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Marker {
    line: u32,
    offset: usize,
}

impl Marker {
    fn new(span: Span) -> Self {
        Self {
            line: span.location_line(),
            offset: span.location_offset(),
        }
    }
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
    fn from(start: Span, end: Span) -> Self {
        Self {
            start: Marker::new(start),
            end: Marker {
                line: end.location_line(),
                offset: end.location_offset() + end.fragment().len()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Placeholder {
    name: String,
    selection: Selection,
    directives: Vec<Directive>,
}

impl Default for Placeholder {
    fn default() -> Self {
        Self {
            name: String::default(),
            selection: Selection::default(),
            directives: Vec::default(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Parsers
/// Parse any character until the end of the line.
/// This will return all characters, except the newline which will be consumed
/// and discarded.
fn parse_until_eol(input: Span) -> IResult<Span, Span> {
    terminated(
        alt((take_until("\n"), rest)),
        alt((tag("\n"), tag(""))),
    )(input)
}

/// Parse a comment starting with either a `#` or `//` and ending with a newline.
///
/// # Example
/// ```rust
/// let input = Span::new("# This is a comment");
/// let (input, meta_comment) = parse_meta_comment(input).unwrap();
/// assert_eq!(input.fragment(), &"");
/// assert_eq!(meta_comment.fragment(), &"This is a comment");
/// ```
fn parse_meta_comment(input: Span) -> IResult<Span, Span> {
    preceded(
        // All comments start with either a `#` or `//` followed by a space(s).
        tuple((alt((tag("#"), tag("//"))), space0)),
        parse_until_eol,
    )(input)
}

/// Parse a key, that starts with an optional `£`, followed by an alphabetic
/// character, then any number of alphanumeric characters, hyphens and
/// underscores.
///
/// # Examples
/// A valid variable, consisting of letters and underscores.
/// ```rust
/// let input = Span::new("£publish_date");
/// let (_, variable) = parse_meta_key(input).unwrap();
/// assert_eq!(variable.fragment(), &"publish_date");
/// ```
/// An invalid example, variables cannot start with a number.
/// ```rust
/// let input = Span::new("£1_to_2");
/// let variable = parse_meta_key(input);
/// assert!(variable.is_err());
/// ```
fn parse_meta_key(input: Span) -> IResult<Span, Span> {
    preceded(
        opt(tag("£")),
        parse_variable_name
    )(input)
}

/// Parse any number of characters until the end of the line or string.
///
/// # Example
/// ```rust
/// let input = Span::new("This is a value");
/// let (_, value) = parse_meta_value(input).unwrap();
/// assert_eq!(value.fragment(), &"This is a value");
/// ```
fn parse_meta_value(input: Span) -> IResult<Span, Span> {
    // The value of the variable, everything after the equals sign.
    // Continue to a newline or the end of the string.
    parse_until_eol(input)
}

/// Parse a key-value pair of meta_key and meta_value.
///
/// # Example
/// ```rust
/// let input = Span::new("£publish_date = 2021-01-01");
/// let (_, meta) = parse_meta_key_value(input).unwrap();
/// assert_eq!(meta.key, "publish_date");
/// assert_eq!(meta.value, "2021-01-01");
/// ```
fn parse_meta_key_value(input: Span) -> IResult<Span, Meta> {
    separated_pair(
        parse_meta_key,
        recognize(tuple((space0, tag("="), space0))),
        parse_meta_value
    )(input)
    .map(|(input, (key, value))| {
        (input, Meta::new(key.fragment(), value.fragment()))
    })
}

/// Parse a line of meta data. This can either be a comment or a key-value pair.
///
/// # Examples
/// Parsing of a comment returns None.
/// ```rust
/// let input = Span::new("# This is a comment");
/// let (_, meta) = parse_meta_line(input).unwrap();
/// assert!(meta.is_none());
/// ```
/// Parsing of a key-value pair returns a Meta object.
/// ```rust
/// let input = Span::new("£publish_date = 2021-01-01");
/// let (_, meta) = parse_meta_line(input).unwrap();
/// assert!(&meta.is_some());
/// let meta = meta.unwrap();
/// assert_eq!(&meta.key, "publish_date");
/// assert_eq!(&meta.value, "2021-01-01");
/// ```
fn parse_meta_line(input: Span) -> IResult<Span, Option<Meta>> {
    let (input, _) = space0(input)?;
    let (input, res) = alt((
        parse_meta_comment.map(|_| None),
        parse_meta_key_value.map(Some),
    ))(input)?;
    let (input, _) = multispace0(input)?;
    Ok((input, res))
}

/// Parse the meta section. This is either a `:meta` or `<meta>` tag surrounding
/// a Vector of parse_meta_line.
///
/// # Example
/// ```rust
/// let input = Span::new(":meta\n// This is the published date\npublish_date = 2021-01-01\n:meta\n# Markdown title");
/// let (input, meta) = parse_meta_section(input).unwrap();
/// assert_eq!(meta.len(), 2);
/// assert_eq!(meta, vec![
///     None,
///     Some(Meta {
///         key: "publish_date".to_string(),
///         value: "2021-01-01".to_string(),
///         directives: vec![],
///     }),
/// ]);
/// assert_eq!(input.fragment(), &"# Markdown title");
/// ```
pub fn parse_meta_section(input: Span) -> IResult<Span, Vec<Option<Meta>>> {
    delimited(
        tuple((multispace0, alt((tag(":meta"), tag("<meta>"))), multispace0)),
        many1(parse_meta_line),
        tuple((multispace0, alt((tag(":meta"), tag("</meta>"))), multispace0)),
    )(input)
}

/// Parse the title of the document. This is either a Markdown title or an HTML
/// heading with the `h1` tag.
///
/// # Examples
/// Using a Markdown heading.
/// ```rust
/// let input = Span::new("# This is the title");
/// let (_, title) = parse_title(input).unwrap();
/// assert_eq!(title.fragment(), &"This is the title");
/// ```
/// Using an HTML heading.
/// ```rust
/// let input = Span::new("<h1>This is the title</h1>");
/// let (_, title) = parse_title(input).unwrap();
/// assert_eq!(title.fragment(), &"This is the title");
/// ```
pub fn parse_title(input: Span) -> IResult<Span, Span> {
    let (input, _) = multispace0(input)?;

    let (input, title) = alt((
        // Either a Markdown title...
        preceded(tuple((tag("#"), space0)), take_till(|c| c == '\n' || c == '\r')),
        // ... or an HTML title.
        delimited(tag("<h1>"), take_until("</h1>"), tag("</h1>"))
    ))(input)?;

    Ok((input.to_owned(), title.to_owned()))
}

/// Rewrite of the nom::is_alphabetic function that takes a char instead.
///
/// # Examples
/// ```rust
/// assert!(is_alphabetic('a'));
/// assert!(is_alphabetic('A'));
/// assert!(!is_alphabetic('1'));
/// assert!(!is_alphabetic('-'));
/// ```
fn is_alphabetic(input: char) -> bool {
    vec!['a'..='z', 'A'..='Z'].into_iter().flatten().find(|c| c == &input).is_some()
}

/// Variable names must start with an alphabetic character, then any number of
/// alphanumeric characters, hyphens and underscores.
///
/// # Examples
/// Variables can consist of letters and underscores.
/// ```rust
/// let input = Span::new("publish_date");
/// let (_, variable) = parse_variable_name(input).unwrap();
/// assert_eq!(variable.fragment(), &"publish_date");
/// ```
/// Variables cannot start with a number or underscore.
/// ```rust
/// let input = Span::new("1_to_2");
/// let variable = parse_variable_name(input);
/// assert!(variable.is_err());
/// ```
fn parse_variable_name(input: Span) -> IResult<Span, Span> {
    recognize(tuple((
        take_while_m_n(1, 1, is_alphabetic),
        many0(alt((alphanumeric1, tag("-"), tag("_")))),
    )))(input)
}

/// Parse a template placeholder variable. This is a `£` followed by a variable
/// name.
///
/// # Examples
/// Variables must start with a `£`.
/// ```rust
/// let input = Span::new("£variable");
/// let (_, variable) = parse_variable(input).unwrap();
/// assert_eq!(variable.fragment(), &"variable");
/// ```
/// Failing to start with a `£` will return an error.
/// ```rust
/// let input = Span::new("variable");
/// let variable = parse_variable(input);
/// assert!(variable.is_err());
/// ```
fn parse_variable(input: Span) -> IResult<Span, Span> {
    preceded(
        tag("£"),
        parse_variable_name
    )(input)
}

/// Parse a placeholder directive, which is any alphabetic character.
/// Optionally, there can be arguments provided following a colon.
///
/// # Examples
/// A simple directive with no arguments.
/// ```rust
/// let input = Span::new("uppercase");
/// let (_, directive) = parse_placeholder_directive_key(input).unwrap();
/// assert_eq!(directive, Some(Directive::Uppercase));
/// ```
fn parse_placeholder_directive_enum(input: Span) -> IResult<Span, Option<Directive>> {
    let (input, name) = take_while(|c| is_alphabetic(c))(input)?;
    let (input, _) = opt(tuple((multispace0, tag(":"), multispace0)))(input)?;

    let (input, arg) = opt(take_while(|c| c != '|' && c != '}'))(input)?;

    let directive = match name.to_ascii_lowercase().as_str() {
        "date" => arg.and_then(|arg| Some(Directive::Date(arg.to_string()))),
        "lowercase" => Some(Directive::Lowercase),
        "uppercase" => Some(Directive::Uppercase),
        "markdown"  => Some(Directive::Markdown),
        _ => None,
    };
    Ok((input, directive))
}

/// Parses the placeholder directive preceded by a pipe character.
///
/// # Example
/// ```rust
/// let input = Span::new(" | uppercase }}");
/// let (input, placeholder) = parse_placeholder_directive(input).unwrap();
/// assert_eq!(placeholder, Some(Directive::Uppercase));
/// assert_eq!(input.fragment(), &" }}");
/// ```
fn parse_placeholder_directive(input: Span) -> IResult<Span, Option<Directive>> {
    preceded(
        tuple((multispace0, tag("|"), multispace0)),
        parse_placeholder_directive_enum
    )(input)
}

/// Parse a template placeholder. This is a variable name, surrounded by `{{`
/// and `}}`.
/// Whitespace is optional.
///
/// # Examples
/// A simple placeholder.
/// ```rust
/// let input = Span::new("{{ £variable }}");
/// let (_, placeholder) = parse_placeholder(input).unwrap();
/// assert_eq!(placeholder.name.as_str(), "variable");
/// assert_eq!(placeholder.selection.start.offset, 0);
/// assert_eq!(placeholder.selection.end.offset, 16);
/// ```
///
/// A placeholder without whitespace.
/// ```rust
/// let input = Span::new("{{£variable}}");
/// let (_, placeholder) = parse_placeholder(input).unwrap();
/// assert_eq!(placeholder.name.as_str(), "variable");
/// assert_eq!(placeholder.selection.start.offset, 0);
/// assert_eq!(placeholder.selection.end.offset, 14);
/// ```
fn parse_placeholder(input: Span) -> IResult<Span, Placeholder> {
    tuple((
        tuple((tag("{{"), multispace0)),
        parse_variable,
        many0(parse_placeholder_directive),
        tuple((multispace0, tag("}}"))),
    ))(input)
    .map(|(input, (start, variable, mut directives, end))| {
        // By default, £content will always be parsed as Markdown.
        if variable.to_ascii_lowercase().as_str() == "content" && !directives.contains(&Some(Directive::Markdown)) {
            directives.push(Some(Directive::Markdown));
        }

        (input, Placeholder {
            name: variable.to_string(),
            directives: directives.into_iter().filter_map(|d| d).collect(),
            selection: Selection::from(start.0, end.1)
        })
    })
}

/// Parse a template consuming - and discarding - any character, and stopping at
/// the first matched placeholder, returning it in full.
///
/// # Example
/// ```rust
/// let input = Span::new("Hello, {{ £name }}!");
/// let (input, placeholders) = take_till_placeholder(input).expect("to parse input");
/// assert_eq!(input.fragment(), &"!");
/// assert_eq!(placeholders.name.as_str(), "name");
/// ```
fn take_till_placeholder(input: Span) -> IResult<Span, Placeholder> {
    many_till(anychar, parse_placeholder)(input)
    // Map to remove anychar's captures.
    .map(|(input, (_, placeholder))| (input, placeholder))
}

/// Consume an entire string, and return a Vector of a tuple; where the first
/// element is a String of the variable name, and the second element is the
/// Placeholder.
///
/// # Example
/// ```rust
/// let input = Span::new("Hello, {{ £name }}!");
/// let placeholders = parse_placeholder_locations(input).unwrap();
/// assert_eq!(placeholders.len(), 1);
/// assert_eq!(placeholders[0].name.as_str(), "name");
/// assert_eq!(placeholders[0].selection.start.offset, 7);
/// assert_eq!(placeholders[0].selection.end.offset, 19);
/// ```
fn parse_placeholder_locations(input: Span) -> Result<Vec<Placeholder>, anyhow::Error> {
    let (_, mut placeholders) = many0(take_till_placeholder)(input).unwrap_or((input, Vec::new()));

    // Sort in reverse so that when we replace each placeholder, the offsets do
    // not affect offsets after this point.
    placeholders.sort_by(|a, b| b.selection.start.offset.cmp(&a.selection.start.offset));

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
/// let original = "Hello, World!";
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

/// Creates a HashMap of key-value pairs from meta values.
///
/// # Arguments
/// * `markdown` - A LocatedSpan of the markdown file.
/// * `meta_values` - An optional vector of Meta values.
///
/// # Returns
/// Convert the meta_values into a HashMap, then parse the title and content
/// from the markdown file.
///
/// # Example
/// ```
/// let markdown = Span::new(":meta\nauthor = John Doe\n:meta\n# Markdown title\nContent paragraph");
/// let (markdown, meta_values) = parse_meta_section(markdown).unwrap_or((markdown, vec![]));
/// let variables = create_variables(markdown, meta_values).expect("to create variables");
/// assert_eq!(variables.get("title").unwrap(), "Markdown title");
/// assert_eq!(variables.get("author").unwrap(), "John Doe");
/// assert_eq!(variables.get("content").unwrap(), "<h1>Markdown title</h1>\n<p>Content paragraph</p>");
/// ```
fn create_variables(markdown: Span, meta_values: Vec<Option<Meta>>) -> Result<HashMap<String, String>, anyhow::Error> {
    let mut variables: HashMap<String, String> = meta_values
        .into_iter()
        .filter_map(|meta| {
            if let Some(meta) = meta {
                Some((meta.key.to_owned(), meta.value.to_owned()))
            } else {
                None
            }
        })
        .collect();

    // Make sure that we have a title and content variable.
    if !variables.contains_key("title") {
        if let Ok(title) = parse_title(markdown) {
            let (_, title) = title;
            variables.insert("title".to_string(), title.to_string());
        } else {
            return Err(anyhow!("Missing title"));
        }
    }
    if !variables.contains_key("content") {
        let content = markdown.fragment().trim().to_string();
        variables.insert("content".to_string(), content);
    }

    Ok(variables)
}




fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let template = cli.template;

    // Check that the actual template exists.
    if !template.try_exists().map_err(|_| anyhow!("The template could not be found."))? {
       Err(anyhow!("The template file does not exist."))?;
    };
    let template = std::fs::read_to_string(&template)?;
    let template = Span::new(&template);

    // Get only existing markdowns.
    let markdown_urls: Vec<PathBuf> = cli.markdowns
        .into_iter()
        .filter(|file| file.exists() && file.extension().unwrap_or_default() == "md")
        .collect();
    let markdowns: Vec<String> = markdown_urls
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect();
    let markdowns: Vec<(String, PathBuf)> = markdowns.into_iter().zip(markdown_urls).collect();

    // All placeholders that are present in the template.
    let mut placeholders = parse_placeholder_locations(template)?;
    placeholders.sort_by(|a, b| b.selection.start.offset.cmp(&a.selection.start.offset));

    for (markdown, markdown_url) in &markdowns {
        let markdown = Span::new(markdown);
        let mut html_doc = template.fragment().to_string();

        // Parse the meta values, and combine them with the title and content of
        // the markdown file.
        let (markdown, meta_values) = parse_meta_section(markdown).unwrap_or((markdown, vec![]));
        let variables: HashMap<String, String> = create_variables(markdown, meta_values)?;

        // Check for unused variables.
        let placeholder_keys = placeholders.iter().map(|p| &p.name).collect::<Vec<&String>>();
        let unused_variables = variables.keys().filter(|key| !placeholder_keys.contains(key)).collect::<Vec<&String>>();
        if !unused_variables.is_empty() {
            println!(
                "Warning: Unused variable{} in '{}': {}",
                if &unused_variables.len() == &(1 as usize) { "" } else { "s" },
                &markdown_url.to_string_lossy(),
                unused_variables.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", ")
            );
        }

        for placeholder in &placeholders {
            if let Some(variable) = variables.get(&placeholder.name) {
                // Used to deref the variable.
                let mut variable = variable.to_owned();

                for directive in &placeholder.directives {
                    variable = match directive {
                        Directive::Lowercase => variable.to_lowercase(),
                        Directive::Uppercase => variable.to_uppercase(),
                        Directive::Markdown  => {
                            markdown::to_html_with_options(&variable, &markdown::Options {
                                compile: markdown::CompileOptions {
                                    allow_dangerous_html: true,
                                    allow_dangerous_protocol: false,
                                    ..Default::default()
                                },
                                ..Default::default()
                            }).unwrap_or_default()
                        }
                        _ => unimplemented!(),
                    }
                }

                html_doc = replace_substring(&html_doc, placeholder.selection.start.offset, placeholder.selection.end.offset, &variable);
            } else {
                let url = markdown_url.to_str().unwrap_or_default();
                return Err(anyhow!("Missing variable '{}' in markdown '{}'.", &placeholder.name, url));
            }
        }

        // Add newlines before each heading element, because I'd like the HTML
        // to be easy to read.
        for h in 2..6 {
            let h = format!("<h{h}>");
            html_doc = html_doc.replace(&h, &format!("\n{h}"));
        };

        // Get the output path where the `.md` is replaced with `.html`.
        let output_path = match cli.output_dir.clone() {
            Some(path) => path.join(markdown_url.with_extension("html").file_name().unwrap()),
            None => markdown_url.with_extension("html"),
        };

        // Create all folders from the path.
        if let Some(path) = output_path.parent() {
            if !path.exists() {
                fs::create_dir_all(path)?;
            }
        }

        fs::write(output_path, html_doc)?;
    }

    Ok(())
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_until_eol() {
        let input = Span::new("This is the first line\nThis is the second line.");
        let (input, line) = parse_until_eol(input).expect("to parse line");
        assert_eq!(line.fragment(), &"This is the first line");
        // Notice that line has consumed the newline, but it is not returned.
        assert_eq!(input.fragment(), &"This is the second line.");

        let (input, line) = parse_until_eol(input).expect("to parse line");
        assert_eq!(line.fragment(), &"This is the second line.");
        // This also works even if there is no newline character.
        assert_eq!(input.fragment(), &"");
    }

    #[test]
    fn can_parse_variable() {
        let input = Span::new("£content }}");
        let variable = parse_variable(input);
        assert!(variable.is_ok());

        let (input, variable) = variable.unwrap();
        assert_eq!(variable.fragment(), &"content");
        assert_eq!(input.fragment(), &" }}");
    }

    #[test]
    fn can_parse_variable_with_one_letter() {
        let input = Span::new("£a }}");
        let variable = parse_variable(input);
        assert!(variable.is_ok());

        let (input, variable) = variable.unwrap();
        assert_eq!(variable.fragment(), &"a");
        assert_eq!(input.fragment(), &" }}");
    }

    #[test]
    fn can_parse_variable_with_underscore() {
        let input = Span::new("£publish_date }}");
        let (input, variable) = parse_variable(input).unwrap();

        assert_eq!(variable.fragment(), &"publish_date");
        assert_eq!(input.fragment(), &" }}");
    }

    #[test]
    fn cannot_parse_variable_starting_with_number() {
        let input = Span::new("£1_to_2");
        let variable = parse_variable(input);
        assert!(variable.is_err());
    }

    #[test]
    fn cannot_parse_variable_starting_with_underscore() {
        let input = Span::new("£_author");
        let variable = parse_variable(input);
        assert!(variable.is_err());
    }

    #[test]
    fn can_parse_meta_comment_slash() {
        let input = Span::new("// This is a comment");
        let (input, meta_comment) = parse_meta_comment(input).expect("to parse comment");

        assert_eq!(input.fragment(), &"");
        assert_eq!(meta_comment.fragment(), &"This is a comment");
    }

    #[test]
    fn can_parse_meta_comment_hash() {
        let input = Span::new("# This is a comment");
        let (input, meta_comment) = parse_meta_comment(input).expect("to parse comment");

        assert_eq!(input.fragment(), &"");
        assert_eq!(meta_comment.fragment(), &"This is a comment");
    }

    #[test]
    fn can_parse_meta_comment_before_key_value() {
        let input = Span::new("// This is a comment\ntitle = My Title");
        let (input, meta_comment) = parse_meta_comment(input).expect("to parse comment");
        assert_eq!(meta_comment.fragment(), &"This is a comment");

        let (input, meta) = parse_meta_key_value(input).expect("to parse key value");
        assert_eq!(meta.key, "title".to_string());
        assert_eq!(meta.value, "My Title".to_string());

        assert_eq!(input.fragment(), &"");
    }

    #[test]
    fn can_parse_placeholder() {
        let input = Span::new("{{ £content }}\nTemplate content");
        let parsed_placeholder = parse_placeholder(input);

        assert!(parsed_placeholder.is_ok());

        let (input, placeholder) = parsed_placeholder.unwrap();
        assert_eq!(placeholder.name, "content".to_string());
        assert_eq!(input.fragment(), &"\nTemplate content");
    }

    #[test]
    fn can_parse_md_title() {
        let markdown = Span::new("# My Title\nMy content");
        let parsed_title = parse_title(markdown);

        assert!(parsed_title.is_ok());

        let (input, title) = parsed_title.unwrap();
        assert_eq!(title.fragment(), &"My Title");
        assert_eq!(input.fragment(), &"\nMy content");
    }

    #[test]
    fn can_parse_html_title() {
        // Deliberately include spaces at the start of this line.
        let markdown = Span::new("    <h1>My Title</h1>\nMy content");
        let parsed_title = parse_title(markdown);

        assert!(parsed_title.is_ok());

        let (input, title) = parsed_title.unwrap();
        assert_eq!(title.fragment(), &"My Title");
        assert_eq!(input.fragment(), &"\nMy content");
    }

    #[test]
    fn can_parse_meta_value() {
        let input = Span::new("title = My Title");
        let (_, meta) = parse_meta_key_value(input).expect("to parse meta key-value");
        assert_eq!(meta, Meta::new("title", "My Title"));
    }

    #[test]
    fn can_parse_meta_value_with_underscore() {
        let input = Span::new("publish_date = 2024-01-01");
        dbg!(input);
        let (_, meta) = parse_meta_key_value(input).expect("to parse meta key-value");
        assert_eq!(meta, Meta::new("publish_date", "2024-01-01"));
    }

    #[test]
    fn can_parse_meta_value_with_prefix() {
        let input = Span::new("£publish_date = 2024-01-01");
        dbg!(input);
        let (_, meta) = parse_meta_key_value(input).expect("to parse meta key-value");
        assert_eq!(meta, Meta::new("publish_date", "2024-01-01"));
    }

    #[test]
    fn can_parse_metadata_colon() {
        let input = Span::new(":meta\ntitle = Meta title\nauthor = John Doe\n:meta\n# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

        assert_eq!(meta, vec![
            Some(Meta::new("title", "Meta title")),
            Some(Meta::new("author", "John Doe")),
        ]);

        assert_eq!(input.fragment(), &"# Markdown title\nThis is my content");
    }

    #[test]
    fn can_parse_metadata_tag() {
        let input = Span::new("<meta>\ntitle = Meta title\nauthor = John Doe\n</meta>\n# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

        assert_eq!(meta, vec![
            Some(Meta::new("title", "Meta title")),
            Some(Meta::new("author", "John Doe")),
        ]);

        assert_eq!(input.fragment(), &"# Markdown title\nThis is my content");
    }

    #[test]
    fn can_parse_when_no_meta_section() {
        let input = Span::new("# Markdown title\nThis is my content");
        let (input, meta) = opt(parse_meta_section)(input).expect("to parse the meta values");

        assert!(meta.is_none());
        assert_eq!(input.fragment(), &"# Markdown title\nThis is my content");
    }

    #[test]
    fn can_parse_meta_section_with_comments() {
        let input = Span::new(":meta\n// This is an author\nauthor = John Doe\n# This is the publish date\npublish_date = 2024-01-01\n:meta\n# Markdown title\nThis is my content");
        let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

        // We get None, Some, None, Some.
        assert!(meta.len() == 4);
        // Then filter and unwrap.
        let meta: Vec<Meta> = meta.into_iter().filter_map(|m| m).collect();

        assert_eq!(meta, vec![
            Meta::new("author", "John Doe"),
            Meta::new("publish_date", "2024-01-01"),
        ]);

        assert_eq!(input.fragment(), &"# Markdown title\nThis is my content");
    }

    #[test]
    fn can_parse_placeholders() {
        let input = Span::new("<h1>{{ £title }}\n<p>{{ £content }}");
        let placeholders = parse_placeholder_locations(input).expect("to parse placeholders");

        // Placeholders are returned in reverse order because we replace from
        // the end of the string.
        // This is to ensure that offsets are not skewed with each replacement.
        assert_eq!(placeholders.len(), 2);
        assert_eq!(placeholders.iter().map(|p| &p.name).collect::<Vec<&String>>(), vec![
            "content",
            "title",
        ]);

        assert_eq!(placeholders[0].selection.start.offset, 21);
        assert_eq!(placeholders[0].name, "content".to_string());

        assert_eq!(placeholders[1].selection.start.offset, 4);
        assert_eq!(placeholders[1].name, "title".to_string());
    }

    #[test]
    fn can_parse_when_no_placeholders() {
        let input = Span::new("<h1>My Title\n<p>My content");
        let placeholders = parse_placeholder_locations(input).expect("to parse empty list");
        assert_eq!(placeholders, vec![]);
    }

    #[test]
    fn can_parse_placeholder_with_no_directive() {
        // Directives are case insensitive.
        let input = Span::new("<p>{{ £variable }}</p>");
        let placeholders = parse_placeholder_locations(input).expect("to parse placeholder");

        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].name, "variable".to_string());
        assert_eq!(placeholders[0].selection.start.offset, 3);
        assert_eq!(placeholders[0].selection.end.offset, 19);
        assert_eq!(placeholders[0].directives, vec![]);
    }

    #[test]
    fn can_parse_placeholder_uppercase_directive() {
        // Directives are case insensitive.
        let input = Span::new("<p>{{ £variable | UPPERCASE }}</p>");
        let placeholders = parse_placeholder_locations(input).expect("to parse placeholder");

        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].name, "variable".to_string());
        assert_eq!(placeholders[0].selection.start.offset, 3);
        assert_eq!(placeholders[0].selection.end.offset, 31);
        assert_eq!(placeholders[0].directives, vec![Directive::Uppercase]);
    }

    #[test]
    fn can_parse_placeholder_lowercase_directive() {
        let input = Span::new("<p>{{ £variable | lowercase }}</p>");
        let placeholders = parse_placeholder_locations(input).expect("to parse placeholder");

        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].name, "variable".to_string());
        assert_eq!(placeholders[0].selection.start.offset, 3);
        assert_eq!(placeholders[0].selection.end.offset, 31);
        assert_eq!(placeholders[0].directives, vec![Directive::Lowercase]);
    }

    #[test]
    fn can_parse_two_placeholder_directives() {
        let input = Span::new("<p>{{ £title | uppercase | lowercase }}</p>");
        let placeholders = parse_placeholder_locations(input).expect("parse placeholders");

        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].name, "title".to_string());
        assert_eq!(placeholders[0].selection.start.offset, 3);
        assert_eq!(placeholders[0].selection.end.offset, 40);
        assert_eq!(placeholders[0].directives, vec![Directive::Uppercase, Directive::Lowercase]);
    }

    #[test]
    fn can_parse_placeholder_directive_with_arg() {
        let input = Span::new("date: dd-mmm-yyyy");
        let (_, directive) = parse_placeholder_directive_enum(input).expect("to parse directive");
        assert!(directive.is_some());
        let directive = directive.unwrap();
        assert_eq!(directive, Directive::Date("dd-mmm-yyyy".to_string()));
    }

    #[test]
    fn can_replace_placeholder_from_meta() {
        let input = Span::new("<meta>\ntitle = Meta title\n£author = John Doe\n</meta>\n# Markdown title\nThis is my content");
        let template = Span::new("<html>\n<head>\n<title>{{ £title }}</title>\n</head>\n<body>\n<h1>{{ £title }}</h1>\n<small>By {{ £author }}</small>\n<section>{{ £content }}</section>\n</body>\n</html>");

        let mut placeholders = parse_placeholder_locations(template).expect("to parse placeholders");
        placeholders.sort_by(|a, b| b.selection.start.offset.cmp(&a.selection.start.offset));

        let mut placeholder_title_iter = placeholders.iter().filter(|p| &p.name == "title");
        assert!(placeholder_title_iter.clone().count() == 2);
        assert_eq!(placeholder_title_iter.next().expect("title to exist").selection, Selection {
            start: Marker { line: 6, offset: 62 },
            end: Marker { line: 6, offset: 75 },
        });
        assert_eq!(placeholder_title_iter.next().expect("title to exist").selection, Selection {
            start: Marker { line: 3, offset: 21 },
            end: Marker { line: 3, offset: 34 },
        });

        assert_eq!(placeholders.iter().filter(|p| &p.name == "content").next().expect("content to exist").selection, Selection {
            start: Marker { line: 8, offset: 123 },
            end: Marker { line: 8, offset: 138 },
        });

        assert_eq!(placeholders.iter().filter(|p| &p.name == "author").next().expect("author to exist").selection, Selection {
            start: Marker { line: 7, offset: 91 },
            end: Marker { line: 7, offset: 105 },
        });

        let (markdown, meta_values) = opt(parse_meta_section)(input).unwrap_or((input, Some(vec![])));
        assert!(meta_values.is_some());

        // Unwrap, to peek the values, then re-wrap.
        let meta_values = meta_values.unwrap_or_default();
        assert_eq!(meta_values, vec![
            Some(Meta::new("title", "Meta title")),
            Some(Meta::new("author", "John Doe")),
        ]);
        let variables: HashMap<String, String> = create_variables(markdown, meta_values).expect("to create variables");

        let mut html_doc = template.to_string();
        for placeholder in &placeholders {
            if let Some(variable) = variables.get(&placeholder.name) {
                html_doc = replace_substring(&html_doc, placeholder.selection.start.offset, placeholder.selection.end.offset, &variable);
            } else {
                assert!(false);
            }
        }

        assert_eq!(html_doc, "<html>\n<head>\n<title>Meta title</title>\n</head>\n<body>\n<h1>Meta title</h1>\n<small>By John Doe</small>\n<section><h1>Markdown title</h1>\n<p>This is my content</p></section>\n</body>\n</html>");
    }
}
