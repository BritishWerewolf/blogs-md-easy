use anyhow::anyhow;
use std::collections::HashMap;
use nom::{branch::alt, bytes::complete::{tag, take_till, take_until, take_while, take_while_m_n}, character::complete::{alphanumeric1, anychar, multispace0, space0}, combinator::{opt, recognize, rest}, multi::{many0, many1, many_till}, sequence::{delimited, preceded, separated_pair, terminated, tuple}, IResult, Parser};
use nom_locate::LocatedSpan;

////////////////////////////////////////////////////////////////////////////////
// Structs and types
pub type Span<'a> = LocatedSpan<&'a str>;

#[derive(Clone, Debug, PartialEq)]
pub enum Directive {
    Date {
        format: String,
    },
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
pub struct Marker {
    pub line: u32,
    pub offset: usize,
}

impl Marker {
    pub fn new(span: Span) -> Self {
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
pub struct Selection {
    pub start: Marker,
    pub end: Marker,
}

impl Selection {
    pub fn from(start: Span, end: Span) -> Self {
        Self {
            start: Marker::new(start),
            end: Marker {
                line: end.location_line(),
                offset: end.location_offset() + end.fragment().len()
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Placeholder {
    pub name: String,
    pub selection: Selection,
    pub directives: Vec<Directive>,
}


////////////////////////////////////////////////////////////////////////////////
// Parsers
/// Parse any character until the end of the line.
/// This will return all characters, except the newline which will be consumed
/// and discarded.
pub fn parse_until_eol(input: Span) -> IResult<Span, Span> {
    terminated(
        alt((take_until("\n"), rest)),
        alt((tag("\n"), tag(""))),
    )(input)
}

/// Parse a comment starting with either a `#` or `//` and ending with a newline.
///
/// # Example
/// ```rust
/// use blogs_md_easy::{parse_meta_comment, Span};
///
/// let input = Span::new("# This is a comment");
/// let (input, meta_comment) = parse_meta_comment(input).unwrap();
/// assert_eq!(input.fragment(), &"");
/// assert_eq!(meta_comment.fragment(), &"This is a comment");
/// ```
pub fn parse_meta_comment(input: Span) -> IResult<Span, Span> {
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
/// use blogs_md_easy::{parse_meta_key, Span};
///
/// let input = Span::new("£publish_date");
/// let (_, variable) = parse_meta_key(input).unwrap();
/// assert_eq!(variable.fragment(), &"publish_date");
/// ```
/// An invalid example, variables cannot start with a number.
/// ```rust
/// use blogs_md_easy::{parse_meta_key, Span};
///
/// let input = Span::new("£1_to_2");
/// let variable = parse_meta_key(input);
/// assert!(variable.is_err());
/// ```
pub fn parse_meta_key(input: Span) -> IResult<Span, Span> {
    preceded(
        opt(tag("£")),
        parse_variable_name
    )(input)
}

/// Parse any number of characters until the end of the line or string.
///
/// # Example
/// ```rust
/// use blogs_md_easy::{parse_meta_value, Span};
///
/// let input = Span::new("This is a value");
/// let (_, value) = parse_meta_value(input).unwrap();
/// assert_eq!(value.fragment(), &"This is a value");
/// ```
pub fn parse_meta_value(input: Span) -> IResult<Span, Span> {
    // The value of the variable, everything after the equals sign.
    // Continue to a newline or the end of the string.
    parse_until_eol(input)
}

/// Parse a key-value pair of meta_key and meta_value.
///
/// # Example
/// ```rust
/// use blogs_md_easy::{parse_meta_key_value, Span};
///
/// let input = Span::new("£publish_date = 2021-01-01");
/// let (_, meta) = parse_meta_key_value(input).unwrap();
/// assert_eq!(meta.key, "publish_date");
/// assert_eq!(meta.value, "2021-01-01");
/// ```
pub fn parse_meta_key_value(input: Span) -> IResult<Span, Meta> {
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
/// use blogs_md_easy::{parse_meta_line, Span};
///
/// let input = Span::new("# This is a comment");
/// let (_, meta) = parse_meta_line(input).unwrap();
/// assert!(meta.is_none());
/// ```
/// Parsing of a key-value pair returns a Meta object.
/// ```rust
/// use blogs_md_easy::{parse_meta_line, Span};
///
/// let input = Span::new("£publish_date = 2021-01-01");
/// let (_, meta) = parse_meta_line(input).unwrap();
/// assert!(&meta.is_some());
/// let meta = meta.unwrap();
/// assert_eq!(&meta.key, "publish_date");
/// assert_eq!(&meta.value, "2021-01-01");
/// ```
pub fn parse_meta_line(input: Span) -> IResult<Span, Option<Meta>> {
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
/// use blogs_md_easy::{parse_meta_section, Meta, Span};
///
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
    alt((
        // I can't think of a more elegant solution for ensuring the pairs match
        // one another. The previous solution could open with `:meta` and close
        // with `</meta>` for example.
        delimited(
            tuple((multispace0, tag(":meta"), multispace0)),
            many1(parse_meta_line),
            tuple((multispace0, tag(":meta"), multispace0)),
        ),
        delimited(
            tuple((multispace0, tag("<meta>"), multispace0)),
            many1(parse_meta_line),
            tuple((multispace0, tag("</meta>"), multispace0)),
        ),
    ))(input)
}

/// Parse the title of the document. This is either a Markdown title or an HTML
/// heading with the `h1` tag.
///
/// # Examples
/// Using a Markdown heading.
/// ```rust
/// use blogs_md_easy::{parse_title, Span};
///
/// let input = Span::new("# This is the title");
/// let (_, title) = parse_title(input).unwrap();
/// assert_eq!(title.fragment(), &"This is the title");
/// ```
/// Using an HTML heading.
/// ```rust
/// use blogs_md_easy::{parse_title, Span};
///
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
/// use blogs_md_easy::is_alphabetic;
///
/// assert!(is_alphabetic('a'));
/// assert!(is_alphabetic('A'));
/// assert!(!is_alphabetic('1'));
/// assert!(!is_alphabetic('-'));
/// ```
pub fn is_alphabetic(input: char) -> bool {
    vec!['a'..='z', 'A'..='Z'].into_iter().flatten().any(|c| c == input)
}

/// Variable names must start with an alphabetic character, then any number of
/// alphanumeric characters, hyphens and underscores.
///
/// # Examples
/// Variables can consist of letters and underscores.
/// ```rust
/// use blogs_md_easy::{parse_variable_name, Span};
///
/// let input = Span::new("publish_date");
/// let (_, variable) = parse_variable_name(input).unwrap();
/// assert_eq!(variable.fragment(), &"publish_date");
/// ```
/// Variables cannot start with a number or underscore.
/// ```rust
/// use blogs_md_easy::{parse_variable_name, Span};
///
/// let input = Span::new("1_to_2");
/// let variable = parse_variable_name(input);
/// assert!(variable.is_err());
/// ```
pub fn parse_variable_name(input: Span) -> IResult<Span, Span> {
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
/// use blogs_md_easy::{parse_variable, Span};
///
/// let input = Span::new("£variable");
/// let (_, variable) = parse_variable(input).unwrap();
/// assert_eq!(variable.fragment(), &"variable");
/// ```
/// Failing to start with a `£` will return an error.
/// ```rust
/// use blogs_md_easy::{parse_variable, Span};
///
/// let input = Span::new("variable");
/// let variable = parse_variable(input);
/// assert!(variable.is_err());
/// ```
pub fn parse_variable(input: Span) -> IResult<Span, Span> {
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
/// use blogs_md_easy::{parse_placeholder_directive_enum, Directive, Span};
///
/// let input = Span::new("uppercase");
/// let (_, directive) = parse_placeholder_directive_enum(input).unwrap();
/// assert_eq!(directive, Some(Directive::Uppercase));
/// ```
pub fn parse_placeholder_directive_enum(input: Span) -> IResult<Span, Option<Directive>> {
    let (input, name) = take_while(is_alphabetic)(input)?;
    let (input, _) = opt(tuple((multispace0, tag(":"), multispace0)))(input)?;

    let (input, arg) = opt(take_while(|c| c != '|' && c != '}'))(input)?;

    let directive = match name.to_ascii_lowercase().as_str() {
        "date" => arg.map(|arg| Directive::Date {
            format: arg.to_string()
        }),
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
/// use blogs_md_easy::{parse_placeholder_directive, Directive, Span};
///
/// let input = Span::new(" | uppercase }}");
/// let (input, placeholder) = parse_placeholder_directive(input).unwrap();
/// assert_eq!(placeholder, Some(Directive::Uppercase));
/// assert_eq!(input.fragment(), &"}}");
/// ```
pub fn parse_placeholder_directive(input: Span) -> IResult<Span, Option<Directive>> {
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
/// use blogs_md_easy::{parse_placeholder, Span};
///
/// let input = Span::new("{{ £variable }}");
/// let (_, placeholder) = parse_placeholder(input).unwrap();
/// assert_eq!(placeholder.name.as_str(), "variable");
/// assert_eq!(placeholder.selection.start.offset, 0);
/// assert_eq!(placeholder.selection.end.offset, 16);
/// ```
///
/// A placeholder without whitespace.
/// ```rust
/// use blogs_md_easy::{parse_placeholder, Span};
///
/// let input = Span::new("{{£variable}}");
/// let (_, placeholder) = parse_placeholder(input).unwrap();
/// assert_eq!(placeholder.name.as_str(), "variable");
/// assert_eq!(placeholder.selection.start.offset, 0);
/// assert_eq!(placeholder.selection.end.offset, 14);
/// ```
pub fn parse_placeholder(input: Span) -> IResult<Span, Placeholder> {
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
            directives: directives.into_iter().flatten().collect(),
            selection: Selection::from(start.0, end.1)
        })
    })
}

/// Parse a template consuming - and discarding - any character, and stopping at
/// the first matched placeholder, returning it in full.
///
/// # Example
/// ```rust
/// use blogs_md_easy::{take_till_placeholder, Span};
///
/// let input = Span::new("Hello, {{ £name }}!");
/// let (input, placeholders) = take_till_placeholder(input).expect("to parse input");
/// assert_eq!(input.fragment(), &"!");
/// assert_eq!(placeholders.name.as_str(), "name");
/// ```
pub fn take_till_placeholder(input: Span) -> IResult<Span, Placeholder> {
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
/// use blogs_md_easy::{parse_placeholder_locations, Span};
///
/// let input = Span::new("Hello, {{ £name }}!");
/// let placeholders = parse_placeholder_locations(input).unwrap();
/// assert_eq!(placeholders.len(), 1);
/// assert_eq!(placeholders[0].name.as_str(), "name");
/// assert_eq!(placeholders[0].selection.start.offset, 7);
/// assert_eq!(placeholders[0].selection.end.offset, 19);
/// ```
pub fn parse_placeholder_locations(input: Span) -> Result<Vec<Placeholder>, anyhow::Error> {
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
/// use blogs_md_easy::replace_substring;
///
/// let original = "Hello, World!";
/// let start = 7;
/// let end = 12;
/// let replacement = "Rust";
/// let result = replace_substring(original, start, end, replacement);
/// println!("{}", result);  // Prints: "Hello, Rust!"
/// ```
pub fn replace_substring(original: &str, start: usize, end: usize, replacement: &str) -> String {
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
/// use blogs_md_easy::{create_variables, parse_meta_section, Span};
///
/// let markdown = Span::new(":meta\nauthor = John Doe\n:meta\n# Markdown title\nContent paragraph");
/// let (markdown, meta_values) = parse_meta_section(markdown).unwrap_or((markdown, vec![]));
/// let variables = create_variables(markdown, meta_values).expect("to create variables");
/// assert_eq!(variables.get("title").unwrap(), "Markdown title");
/// assert_eq!(variables.get("author").unwrap(), "John Doe");
/// assert_eq!(variables.get("content").unwrap(), "# Markdown title\nContent paragraph");
/// ```
pub fn create_variables(markdown: Span, meta_values: Vec<Option<Meta>>) -> Result<HashMap<String, String>, anyhow::Error> {
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

/// Take a variable, and run it through a Directive function to get the new
/// output.
///
/// # Example
/// ```rust
/// use blogs_md_easy::{render_directive, Directive};
///
/// let variable = "hello, world!".to_string();
/// assert_eq!("HELLO, WORLD!", render_directive(variable, &Directive::Uppercase));
/// ```
pub fn render_directive(variable: String, directive: &Directive) -> String {
    match directive {
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
