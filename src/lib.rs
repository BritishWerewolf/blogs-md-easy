use anyhow::anyhow;
use std::collections::HashMap;
use nom::{branch::alt, bytes::complete::{tag, take_till, take_until, take_while, take_while_m_n}, character::complete::{alphanumeric1, anychar, multispace0, space0}, combinator::{opt, recognize, rest}, multi::{many0, many1, many_till, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated, tuple}, IResult, Parser};
use nom_locate::LocatedSpan;

////////////////////////////////////////////////////////////////////////////////
// Structs and types
pub type Span<'a> = LocatedSpan<&'a str>;

#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    Lowercase,
    Uppercase,
    Markdown,
    Reverse,
    Truncate {
        characters: u8,
        trail: String,
    }
}

#[derive(Debug, PartialEq)]
pub struct Meta {
    pub key: String,
    pub value: String,
    pub filters: Vec<Filter>,
}

impl Meta {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.trim().to_string(),
            value: value.trim().to_string(),
            filters: Vec::new(),
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
    pub filters: Vec<Filter>,
}


////////////////////////////////////////////////////////////////////////////////
// Parsers
/// Parse any character until the end of the line.
/// This will return all characters, except the newline which will be consumed
/// and discarded.
///
/// # Examples
/// When there is no newline.
/// ```rust
/// use blogs_md_easy::{parse_until_eol, Span};
///
/// let input = Span::new("Hello, World!");
/// let (input, until_eol) = parse_until_eol(input).unwrap();
/// assert_eq!(input.fragment(), &"");
/// assert_eq!(until_eol.fragment(), &"Hello, World!");
/// ```
///
/// There is a newline.
/// ```rust
/// use blogs_md_easy::{parse_until_eol, Span};
///
/// let input = Span::new("Hello, World!\nThis is Sparta!");
/// let (input, until_eol) = parse_until_eol(input).unwrap();
/// assert_eq!(input.fragment(), &"This is Sparta!");
/// assert_eq!(until_eol.fragment(), &"Hello, World!");
/// ```
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
/// // Comments are ignored and removed from the Vector.
/// assert_eq!(meta.len(), 1);
/// assert_eq!(meta, vec![
///     Meta {
///         key: "publish_date".to_string(),
///         value: "2021-01-01".to_string(),
///         filters: vec![],
///     },
/// ]);
/// assert_eq!(input.fragment(), &"# Markdown title");
/// ```
pub fn parse_meta_section(input: Span) -> IResult<Span, Vec<Meta>> {
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
    // Filter out None values, leaving only legitimate meta values.
    .map(|(input, res)| {
        // When calling flatten on Option<> types, None values are considered
        // empty iterators and removed, Some values are considered iterators
        // with a single element and are therefore unwrapped and returned.
        (input, res.into_iter().flatten().collect())
    })
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
/// # Example
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

/// A function that checks if a character is valid for a filter name.
/// The filter name is the value before the `=`.
///
/// # Example
/// ```rust
/// use blogs_md_easy::is_filter_name;
///
/// assert!(is_filter_name('a'));
/// assert!(is_filter_name('A'));
/// assert!(is_filter_name('1'));
/// assert!(!is_filter_name('-'));
/// assert!(!is_filter_name(' '));
/// ```
pub fn is_filter_name(input: char) -> bool {
    input.is_alphanumeric() || ['_'].contains(&input)
}

/// A function that checks if a character is valid for a filter argument name.
///
/// # Example
/// ```rust
/// use blogs_md_easy::is_filter_arg;
///
/// assert!(is_filter_arg('a'));
/// assert!(is_filter_arg('A'));
/// assert!(is_filter_arg('1'));
/// assert!(!is_filter_arg('-'));
/// assert!(!is_filter_arg(' '));
/// ```
pub fn is_filter_arg(input: char) -> bool {
    input.is_alphanumeric() || ['_'].contains(&input)
}

/// A function that checks if a character is valid for a filter argument name.
///
/// # Example
/// ```rust
/// use blogs_md_easy::is_filter_value;
///
/// assert!(is_filter_value('a'));
/// assert!(is_filter_value('A'));
/// assert!(is_filter_value('1'));
/// assert!(!is_filter_value('|'));
/// assert!(!is_filter_value(','));
/// assert!(!is_filter_value('{'));
/// assert!(!is_filter_value('}'));
/// ```
pub fn is_filter_value(input: char) -> bool {
    input.is_alphanumeric()
    || ![' ', '|', ',', '{', '}'].contains(&input)
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

/// Parser that will parse exclusively the key-values from after a filter.  \
/// This will return the key (before the `:`) and the value (after the `:`). It
/// will also return a key of `_` if no key was provided.
///
/// # Returns
/// A tuple of the filter name and a vector of key-value pairs.  \
/// If only a value was provided, then the key will be `_`.
///
/// # Examples
/// Ensure that a key-value pair, separated by a colon, can be parsed into a
/// tuple.
/// ```rust
/// use blogs_md_easy::{parse_filter_key_value, Span};
///
/// let input = Span::new("trail: ...");
/// let (_, args) = parse_filter_key_value(input).unwrap();
/// assert_eq!(args, ("trail", "..."));
/// ```
///
/// Ensure that a single value can be parsed into a tuple with a key of `_`.
/// ```rust
/// use blogs_md_easy::{parse_filter_key_value, Span};
///
/// let input = Span::new("20");
/// let (_, args) = parse_filter_key_value(input).unwrap();
/// assert_eq!(args, ("_", "20"));
/// ```
pub fn parse_filter_key_value(input: Span) -> IResult<Span, (&str, &str)> {
    alt((
        // This matches a key-value separated by a colon.
        // Example: `truncate = characters: 20`
        separated_pair(
            take_while(is_filter_arg).map(|arg: Span| *arg.fragment()),
            tuple((space0, tag(":"), space0)),
            take_while(is_filter_value).map(|value: Span| *value.fragment()),
        ),
        // But it's also possible to just provide a value.
        // Example: `truncate = 20`
        take_while(is_filter_value)
        .map(|value: Span| ("_", *value.fragment()))
    ))(input)
}

/// Parser that will parse exclusively the key-values from after a filter.  \
/// The signature of a filter is `filter_name = key1: value1, key2: value2,...`,
/// or just `filter_name = value`.
///
/// # Examples
/// Ensure that a key-value pair, separated by a colon, can be parsed into a
/// tuple.
/// ```rust
/// use blogs_md_easy::{parse_filter_args, Span};
///
/// let input = Span::new("characters: 20, trail: ...");
/// let (_, args) = parse_filter_args(input).unwrap();
/// assert_eq!(args, vec![
///     ("characters", "20"),
///     ("trail", "..."),
/// ]);
/// ```
///
/// Ensure that a single value can be parsed into a tuple with a key of `_`.
/// ```rust
/// use blogs_md_easy::{parse_filter_args, Span};
///
/// let input = Span::new("20");
/// let (_, args) = parse_filter_args(input).unwrap();
/// assert_eq!(args, vec![
///     ("_", "20")
/// ]);
/// ```
pub fn parse_filter_args(input: Span) -> IResult<Span, Vec<(&str, &str)>> {
    separated_list1(
        tuple((space0, tag(","), space0)),
        parse_filter_key_value
    )(input)
}

/// Parse a filter, and optionally its arguments if present.
///
/// # Examples
/// A filter with no arguments.
/// ```rust
/// use blogs_md_easy::{parse_filter, Filter, Span};
///
/// let input = Span::new("lowercase");
/// let (_, filter) = parse_filter(input).unwrap();
/// assert!(matches!(filter, Filter::Lowercase));
/// ```
///
/// A filter with just a value, but no key.  \
/// This will be parsed as a key of `_`, which will then be set to a key of the
/// given enum Struct variant that is deemed the default.  \
/// In the case of `Filter::Truncate`, this will be the `characters`.
/// ```rust
/// use blogs_md_easy::{parse_filter, Filter, Span};
///
/// let input = Span::new("truncate = 20");
/// let (_, filter) = parse_filter(input).unwrap();
/// assert_eq!(filter, Filter::Truncate { characters: 20, trail: "...".to_string() });
/// ```
///
/// A filter with multiple arguments, and given keys.
/// ```rust
/// use blogs_md_easy::{parse_filter, Filter, Span};
///
/// let input = Span::new("truncate = characters: 15, trail:...");
/// let (_, filter) = parse_filter(input).unwrap();
/// assert!(matches!(filter, Filter::Truncate { .. }));
/// assert_eq!(filter, Filter::Truncate {
///     characters: 15,
///     trail: "...".to_string(),
/// });
/// ```
///
/// For some filters, default values are provided, if not present.
/// ```rust
/// use blogs_md_easy::{parse_filter, Filter, Span};
///
/// let input = Span::new("truncate = trail:...");
/// let (_, filter) = parse_filter(input).unwrap();
/// assert!(matches!(filter, Filter::Truncate { .. }));
/// assert_eq!(filter, Filter::Truncate {
///     characters: 20,
///     trail: "...".to_string(),
/// });
/// ```
pub fn parse_filter(input: Span) -> IResult<Span, Filter> {
    separated_pair(
        take_while(is_filter_name),
        opt(tuple((space0, tag("="), space0))),
        opt(parse_filter_args)
    )(input)
    .map(|(input, (name, args))| {
        let args: HashMap<&str, &str> = args.unwrap_or_default().into_iter().collect();

        (input, match name.fragment().to_lowercase().trim() {
            "lowercase" => Filter::Lowercase,
            "uppercase" => Filter::Uppercase,
            "markdown" => Filter::Markdown,
            "reverse" => Filter::Reverse,
            "truncate" => Filter::Truncate {
                // Attempt to get the characters, but if we can't then we use
                // the unnamed value, defined as "_".
                characters: args.get("characters").unwrap_or(
                    args.get("_").unwrap_or(&"20")
                ).parse::<u8>().unwrap_or(20),
                trail: args.get("trail").unwrap_or(&"...").to_string()
            },
            _ => unreachable!(),
        })
    })
}

/// Parsers a list of Filters.
///
/// # Examples
/// A single filter.
/// ```rust
/// use blogs_md_easy::{parse_filters, Filter, Span};
///
/// // As in {{ £my_variable | lowercase }}
/// let input = Span::new("| lowercase");
/// let (_, filters) = parse_filters(input).unwrap();
/// assert!(matches!(filters[0], Filter::Lowercase));
/// ```
///
/// Multiple filters chained together with `|`.
/// ```rust
/// use blogs_md_easy::{parse_filters, Filter, Span};
///
/// // As in {{ £my_variable | lowercase | truncate = trail: ..! }}
/// let input = Span::new("| lowercase | truncate = trail: ..!");
/// let (_, filters) = parse_filters(input).unwrap();
/// assert!(matches!(filters[0], Filter::Lowercase));
/// assert!(matches!(filters[1], Filter::Truncate { .. }));
/// assert_eq!(filters[1], Filter::Truncate {
///     characters: 20,
///     trail: "..!".to_string(),
/// });
/// ```
pub fn parse_filters(input: Span) -> IResult<Span, Vec<Filter>> {
    preceded(
        tuple((space0, tag("|"), space0)),
        separated_list1(tuple((space0, tag("|"), space0)), parse_filter)
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
        opt(parse_filters),
        tuple((multispace0, tag("}}"))),
    ))(input)
    .map(|(input, (start, variable, filters, end))| {
        let mut filters = filters.unwrap_or_default();

        // By default, £content will always be parsed as Markdown.
        if variable.to_ascii_lowercase().as_str() == "content" && !filters.contains(&Filter::Markdown) {
            filters.push(Filter::Markdown);
        }

        (input, Placeholder {
            name: variable.to_string(),
            filters,
            selection: Selection::from(start.0, end.1)
        })
    })
}

/// Parse a string consuming - and discarding - any character, and stopping at
/// the first matched placeholder, returning a Placeholder struct.
///
/// # Example
/// ```rust
/// use blogs_md_easy::{take_till_placeholder, Marker, Placeholder, Selection, Span};
///
/// let input = Span::new("Hello, {{ £name }}!");
/// let (input, placeholder) = take_till_placeholder(input).expect("to parse input");
/// assert_eq!(input.fragment(), &"!");
/// assert_eq!(placeholder, Placeholder {
///     name: "name".to_string(),
///     selection: Selection {
///         start: Marker {
///             line: 1,
///             offset: 7,
///         },
///         end: Marker {
///             line: 1,
///             offset: 19,
///         },
///     },
///     filters: vec![],
/// });
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
pub fn create_variables(markdown: Span, meta_values: Vec<Meta>) -> Result<HashMap<String, String>, anyhow::Error> {
    let mut variables: HashMap<String, String> = meta_values
        .into_iter()
        .map(|meta| (meta.key.to_owned(), meta.value.to_owned()))
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

/// Take a variable, and run it through a Filter function to get the new
/// output.
///
/// # Examples
/// Filter that has no arguments.
/// ```rust
/// use blogs_md_easy::{render_filter, Filter};
///
/// let variable = "hello, world!".to_string();
/// assert_eq!("HELLO, WORLD!", render_filter(variable, &Filter::Uppercase));
/// ```
///
/// Filter that has arguments.
/// ```rust
/// use blogs_md_easy::{render_filter, Filter};
///
/// let variable = "hello, world!".to_string();
/// assert_eq!("hello...", render_filter(variable, &Filter::Truncate { characters: 5, trail: "...".to_string() }));
/// ```
pub fn render_filter(variable: String, filter: &Filter) -> String {
    match filter {
        Filter::Lowercase => variable.to_lowercase(),
        Filter::Uppercase => variable.to_uppercase(),
        Filter::Markdown  => {
            markdown::to_html_with_options(&variable, &markdown::Options {
                compile: markdown::CompileOptions {
                    allow_dangerous_html: true,
                    allow_dangerous_protocol: false,
                    ..Default::default()
                },
                ..Default::default()
            }).unwrap_or_default()
        },
        Filter::Reverse => variable.chars().rev().collect(),
        Filter::Truncate { characters, trail } => {
            let mut new_variable = variable.to_string();
            new_variable.truncate(*characters as usize);
            // Now truncate and append the trail.
            if (variable.len() as u8) > *characters {
                new_variable.push_str(trail);
            }
            new_variable
        },
    }
}
