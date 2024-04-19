use std::{collections::HashMap, error::Error, ops::{Div, Mul}, str::FromStr};
use nom::{branch::alt, bytes::complete::{tag, take_till, take_until, take_while, take_while_m_n}, character::complete::{alphanumeric1, anychar, multispace0, space0}, combinator::{opt, recognize, rest}, multi::{many0, many1, many_till, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated, tuple}, IResult, Parser};
use nom_locate::LocatedSpan;

////////////////////////////////////////////////////////////////////////////////
// Structs and types
/// A [`LocatedSpan`] of a string slice, with lifetime `'a`.
pub type Span<'a> = LocatedSpan<&'a str>;

/// A list of all the available text case `Filter`s.
#[derive(Clone, Debug, PartialEq)]
pub enum TextCase {
    /// Converts a string into lowercase.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "Hello, World!".to_string();
    /// let filter = Filter::Text { case: TextCase::Lower };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "hello, world!");
    /// ```
    Lower,
    /// Converts a string into uppercase.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "Hello, World!".to_string();
    /// let filter = Filter::Text { case: TextCase::Upper };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "HELLO, WORLD!");
    /// ```
    Upper,
    /// Converts a string into title case.
    ///
    /// Every character that supersedes a space or hyphen.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "john doe-bloggs".to_string();
    /// let filter = Filter::Text { case: TextCase::Title };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "John Doe-Bloggs");
    /// ```
    Title,
    /// Converts a string into kebab case.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "kebab case".to_string();
    /// let filter = Filter::Text { case: TextCase::Kebab };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "kebab-case");
    /// ```
    /// Converts a string into kebab case.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "kebab case".to_string();
    /// let filter = Filter::Text { case: TextCase::Kebab };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "kebab-case");
    /// ```
    Kebab,
    /// Converts a string into snake case.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "snake case".to_string();
    /// let filter = Filter::Text { case: TextCase::Snake };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "snake_case");
    /// ```
    Snake,
    /// Converts a string into Pascal case.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "pascal case".to_string();
    /// let filter = Filter::Text { case: TextCase::Pascal };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "PascalCase");
    /// ```
    Pascal,
    /// Converts a string into camel case.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "camel case".to_string();
    /// let filter = Filter::Text { case: TextCase::Camel };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "camelCase");
    /// ```
    Camel,
    /// Converts a string by inverting the case.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "Hello, World!".to_string();
    /// let filter = Filter::Text { case: TextCase::Invert };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "hELLO, wORLD!");
    /// ```
    Invert,
}

impl FromStr for TextCase {
    type Err = String;

    /// Parse a string slice, into a `TextCase`.
    ///
    /// # Examples
    /// ```rust
    /// use blogs_md_easy::TextCase;
    /// // For both lower and upper, the word "case" can be appended.
    /// assert_eq!("lower".parse::<TextCase>(), Ok(TextCase::Lower));
    /// assert_eq!("lowercase".parse::<TextCase>(), Ok(TextCase::Lower));
    ///
    /// // For programming cases, the word case can be appended in that style.
    /// assert_eq!("snake".parse::<TextCase>(), Ok(TextCase::Snake));
    /// assert_eq!("snake_case".parse::<TextCase>(), Ok(TextCase::Snake));
    /// assert_eq!("title".parse::<TextCase>(), Ok(TextCase::Title));
    /// assert_eq!("Title".parse::<TextCase>(), Ok(TextCase::Title));
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lower" | "lowercase" => Ok(Self::Lower),
            "upper" | "uppercase" | "UPPERCASE" => Ok(Self::Upper),
            "title" | "Title" => Ok(Self::Title),
            "kebab" | "kebab-case" => Ok(Self::Kebab),
            "snake" | "snake_case" => Ok(Self::Snake),
            "pascal" | "PascalCase" => Ok(Self::Pascal),
            "camel" | "camelCase" => Ok(Self::Camel),
            "invert" | "inverse" => Ok(Self::Invert),
            _ => Err(format!("Unable to parse TextCase from '{}'", s)),
        }
    }
}

/// Predefined functions names that will be used within [`render_filter`] to
/// convert a value.
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    // Maths filters

    /// Rounds a numeric value up to the nearest whole number.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = "1.234".to_string();
    /// let filter = Filter::Ceil;
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "2");
    /// ```
    Ceil,
    /// Rounds a numeric value down to the nearest whole number.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = "4.567".to_string();
    /// let filter = Filter::Floor;
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "4");
    /// ```
    Floor,
    /// Round a number to a given precision.
    ///
    /// `Default argument: precision`
    ///
    /// # Examples
    /// Precision of 0 to remove decimal place.
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = "1.234".to_string();
    /// let filter = Filter::Round { precision: 0 };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "1");
    /// ```
    ///
    /// Precision of 3 for three decimal places.
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = "1.23456789".to_string();
    /// let filter = Filter::Round { precision: 3 };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "1.235");
    /// ```
    Round {
        /// The number of decimal places to round to.
        /// A half is rounded down.
        ///
        /// `Default: 0`
        ///
        /// # Examples
        /// Providing no arguments.
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span};
        ///
        /// let input = Span::new("round");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Round { .. }));
        /// assert_eq!(filter, Filter::Round { precision: 0 });
        /// ```
        ///
        /// Providing the default argument.
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span};
        ///
        /// let input = Span::new("round = 3");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Round { .. }));
        /// assert_eq!(filter, Filter::Round { precision: 3 });
        /// ```
        ///
        /// Alternatively, it is possible to be more explicit.
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span};
        ///
        /// let input = Span::new("round = precision: 42");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Round { .. }));
        /// assert_eq!(filter, Filter::Round { precision: 42 });
        /// ```
        precision: u8,
    },

    // String filter

    /// Converts a string from Markdown into HTML.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = r#"# Markdown Title
    /// First paragraph.
    ///
    /// [example.com](https://example.com)
    ///
    /// * Unordered list
    ///
    /// 1. Ordered list"#.to_string();
    /// let filter = Filter::Markdown;
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, r#"<h1>Markdown Title</h1>
    /// <p>First paragraph.</p>
    /// <p><a href="https://example.com">example.com</a></p>
    /// <ul>
    /// <li>Unordered list</li>
    /// </ul>
    /// <ol>
    /// <li>Ordered list</li>
    /// </ol>"#);
    /// ```
    Markdown,
    /// Replace a given substring with another. Optionally, limit the number of
    /// replacements from the start of the string.
    ///
    /// `Default argument: find`
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = "Hello, World!".to_string();
    /// let filter = Filter::Replace {
    ///     find: "World".to_string(),
    ///     replacement: "Rust".to_string(),
    ///     limit: None,
    /// };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "Hello, Rust!");
    /// ```
    Replace {
        /// The substring that we are looking for.
        find: String,
        /// The substring that will replace what we `find`.
        replacement: String,
        /// Limit the number of replacements from the start of the string.
        ///
        /// `Default: None`
        ///
        /// # Examples
        /// Without an argument, this will default to doing nothing.
        /// ```rust
        /// use blogs_md_easy::{parse_placeholder, render_filter, Filter, Span};
        ///
        /// let input = Span::new("{{ £greeting | replace }}");
        /// let (_, placeholder) = parse_placeholder(input).unwrap();
        ///
        /// assert!(matches!(placeholder.filters[0], Filter::Replace { .. }));
        /// assert_eq!(placeholder.filters[0], Filter::Replace {
        ///     find: "".to_string(),
        ///     replacement: "".to_string(),
        ///     limit: None,
        /// });
        ///
        /// let greeting = "Hello, World!".to_string();
        /// // Cloning here, only so we can reuse the `greeting` variable in
        /// // assert, to prove that they are identical.
        /// let output = render_filter(greeting.clone(), &placeholder.filters[0]);
        /// assert_eq!(output, greeting);
        /// ```
        ///
        /// Providing the default argument.
        /// In this case the value will be assigned to `find`, and the
        /// `replacement` will be an empty String, essentially removing this
        /// phrase from the string.
        /// ```rust
        /// use blogs_md_easy::{parse_placeholder, render_filter, Filter, Span};
        ///
        /// let input = Span::new("{{ £greeting | replace = World }}");
        /// let (_, placeholder) = parse_placeholder(input).unwrap();
        ///
        /// assert!(matches!(placeholder.filters[0], Filter::Replace { .. }));
        /// assert_eq!(placeholder.filters[0], Filter::Replace {
        ///     find: "World".to_string(),
        ///     replacement: "".to_string(),
        ///     limit: None,
        /// });
        ///
        /// let greeting = "Hello, World!".to_string();
        /// let output = render_filter(greeting, &placeholder.filters[0]);
        /// assert_eq!(output, "Hello, !".to_string());
        /// ```
        ///
        /// Specify the number of replacements.
        /// ```rust
        /// use blogs_md_easy::{parse_placeholder, render_filter, Filter, Span};
        ///
        /// let input = Span::new("{{ £greeting | replace = !, limit: 2 }}");
        /// let (_, placeholder) = parse_placeholder(input).unwrap();
        ///
        /// assert!(matches!(placeholder.filters[0], Filter::Replace { .. }));
        /// assert_eq!(placeholder.filters[0], Filter::Replace {
        ///     find: "!".to_string(),
        ///     replacement: "".to_string(),
        ///     limit: Some(2),
        /// });
        ///
        /// let greeting = "Hello, World!!!".to_string();
        /// let output = render_filter(greeting, &placeholder.filters[0]);
        /// assert_eq!(output, "Hello, World!".to_string());
        /// ```
        ///
        /// Setting all arguments explicitly.
        /// ```rust
        /// use blogs_md_easy::{parse_placeholder, render_filter, Filter, Span};
        ///
        /// let input = Span::new("{{ £greeting | replace = find: World, replacement: Rust, limit: 1 }}");
        /// let (_, placeholder) = parse_placeholder(input).unwrap();
        ///
        /// assert!(matches!(placeholder.filters[0], Filter::Replace { .. }));
        /// assert_eq!(placeholder.filters[0], Filter::Replace {
        ///     find: "World".to_string(),
        ///     replacement: "Rust".to_string(),
        ///     limit: Some(1),
        /// });
        ///
        /// let greeting = "Hello, World! Hello, World!".to_string();
        /// let output = render_filter(greeting, &placeholder.filters[0]);
        /// assert_eq!(output, "Hello, Rust! Hello, World!".to_string());
        /// ```
        limit: Option<u8>,
    },
    /// Reverse a string, character by character.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = "Hello, World!".to_string();
    /// let filter = Filter::Reverse;
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "!dlroW ,olleH");
    /// ```
    Reverse,
    /// Converts text to another format.
    ///
    /// Currently, the only argument is `case`.
    ///
    /// `Default argument: case`
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter, TextCase};
    ///
    /// let input = "Hello, World!".to_string();
    /// let filter = Filter::Text { case: TextCase::Upper };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "HELLO, WORLD!");
    /// ```
    Text {
        /// Specifies the [`TextCase`] that the font should use.
        ///
        /// `Default: lower`
        ///
        /// # Examples
        /// Without an argument, this will default to lowercase.
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span, TextCase};
        ///
        /// let input = Span::new("text");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Text { .. }));
        /// assert_eq!(filter, Filter::Text { case: TextCase::Lower });
        /// ```
        ///
        /// Passing in a case, without an argument is possible too.
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span, TextCase};
        ///
        /// let input = Span::new("text = upper");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Text { .. }));
        /// assert_eq!(filter, Filter::Text { case: TextCase::Upper });
        /// ```
        ///
        /// Alternatively, it is possible to be more explicit.
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span, TextCase};
        ///
        /// let input = Span::new("text = case: snake");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Text { .. }));
        /// assert_eq!(filter, Filter::Text { case: TextCase::Snake });
        /// ```
        case: TextCase,
    },
    /// Truncates a string to a given length, and applies a `trail`ing string,
    /// if the string was truncated.
    ///
    /// `Default argument: characters`
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::{render_filter, Filter};
    ///
    /// let input = "Hello, World!".to_string();
    /// let filter = Filter::Truncate { characters: 5, trail: "...".to_string() };
    /// let output = render_filter(input, &filter);
    ///
    /// assert_eq!(output, "Hello...");
    /// ```
    Truncate {
        /// The number of characters the String will be cut to.
        ///
        /// If this number is greater than the String's length, then nothing
        /// happens to the String.
        ///
        /// `Default: 100`
        ///
        /// # Example
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span};
        ///
        /// let input = Span::new("truncate = trail: --");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Truncate { .. }));
        /// assert_eq!(filter, Filter::Truncate {
        ///     characters: 100,
        ///     trail: "--".to_string(),
        /// });
        /// ```
        characters: u8,
        /// The trailing characters to be appended to a truncated String.
        ///
        /// Due to this being appended, that means that your string will exceed
        /// the characters length.  \
        /// To counter this, you will need to reduce your `characters` value.
        ///
        /// `Default: "..."`
        ///
        /// # Example
        /// ```rust
        /// use blogs_md_easy::{parse_filter, Filter, Span};
        ///
        /// let input = Span::new("truncate = characters: 42");
        /// let (_, filter) = parse_filter(input).unwrap();
        ///
        /// assert!(matches!(filter, Filter::Truncate { .. }));
        /// assert_eq!(filter, Filter::Truncate {
        ///     characters: 42,
        ///     trail: "...".to_string(),
        /// });
        /// ```
        trail: String,
    }
}

/// A simple struct to store the key value pair from within the meta section of
/// a Markdown file.
///
/// # Example
/// ```rust
/// use blogs_md_easy::{parse_meta_line, Meta, Span};
///
/// let input = Span::new("foo = bar");
/// let (_, meta) = parse_meta_line(input).unwrap();
/// // Unwrap because key-values are Some() and comments are None.
/// let meta = meta.unwrap();
/// assert_eq!(meta, Meta::new("foo", "bar"));
/// ```
#[derive(Debug, PartialEq)]
pub struct Meta {
    pub key: String,
    pub value: String,
}

impl Meta {
    /// Trims the `key` and `value` and stores them in the respective values in
    /// this struct.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::Meta;
    ///
    /// let meta_with_space = Meta::new("  foo  ", "  bar  ");
    /// let meta_without_space = Meta::new("foo", "bar");
    /// assert_eq!(meta_with_space, meta_without_space);
    /// ```
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.trim().to_string(),
            value: value.trim().to_string(),
        }
    }
}

/// A position for a Cursor within a [`Span`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub line: u32,
    pub offset: usize,
}

impl Marker {
    /// Extracts the `location_line()` and `location_offset()` from the [`Span`].
    pub fn new(span: Span) -> Self {
        Self {
            line: span.location_line(),
            offset: span.location_offset(),
        }
    }
}

impl Default for Marker {
    /// Create a `Marker` with a `line` of `1` and `offset` of `1`.
    ///
    /// # Example
    /// ```rust
    /// use blogs_md_easy::Marker;
    ///
    /// let marker_default = Marker::default();
    /// let marker_new = Marker { line: 1, offset: 1 };
    /// assert_eq!(marker_default, marker_new);
    /// ```
    fn default() -> Self {
        Self {
            line: 1,
            offset: 1,
        }
    }
}

/// A helper struct that contains a start and end [`Marker`] of a [`Span`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Selection {
    pub start: Marker,
    pub end: Marker,
}

impl Selection {
    /// Generate a new selection from two [`Span`]s.
    ///
    /// The `start` argument will simply extract the `location_line` and
    /// `location_offset` from the [`Span`].
    /// The `end` argument will use the `location_line`, but will set the offset
    /// to the `location_offset` added to the `fragment` length to ensure we
    /// consume the entire match.
    pub fn from(start: Span, end: Span) -> Self {
        Self {
            start: Marker::new(start),
            // We cannot use `new` because we need to account for the string
            // fragment length.
            end: Marker {
                line: end.location_line(),
                offset: end.location_offset() + end.fragment().len()
            }
        }
    }
}

/// A `Placeholder` is a variable that is created within a Template file.
///
/// The syntax for a `Placeholder` is as below.
///
/// `{{ £variable_name[| filter_name[= [key: ]value]...] }}`
///
/// A compulsory `variable_name`, preceded by a `£`.  \
/// Then an optional pipe (`|`) separated list of [`Filter`]s.  \
/// Some filters are just a name, although some have additional arguments.
///
/// For more explanation on what a `Placeholder` looks like inside a template,
/// see [`parse_placeholder`].
///
/// For more explanation on what a [`Filter`] looks like inside a `Placeholder`,
/// see [`parse_filter`].
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
/// When there is a newline.
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

/// Parse the meta section. This is either a `:meta`, `<meta>`, or `<?meta` tag
/// surrounding a Vector of [`parse_meta_line`].
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
            tuple((multispace0, tag("<?"), opt(tag("meta")), multispace0)),
            many1(parse_meta_line),
            tuple((multispace0, tag("?>"), multispace0)),
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

/// Rewrite of the `nom::is_alphabetic` function that takes a char instead.
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
///
/// The filter name is the value before the `=` in a Template.
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
/// This is the string preceding the `=` in the `meta` section.
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

/// A function that checks if a character is valid for a filter argument value.
///
/// This is the string following the `=` in the `meta` section.
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
///
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
///
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

/// Parse a [`Filter`], and optionally its arguments if present.
///
/// # Examples
/// A filter with no arguments.
/// ```rust
/// use blogs_md_easy::{parse_filter, Filter, Span, TextCase};
///
/// let input = Span::new("lowercase");
/// let (_, filter) = parse_filter(input).unwrap();
/// assert!(matches!(filter, Filter::Text { case: TextCase::Lower }));
/// ```
///
/// A filter with just a value, but no key.  \
/// This will be parsed as a key of `_`, which will then be set to a key of the
/// given enum Struct variant that is deemed the default.  \
/// In the case of [`Filter::Truncate`], this will be the `characters`.
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
///     characters: 100,
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
            // Maths filters.
            "ceil" => Filter::Ceil,
            "floor" => Filter::Floor,
            "round" => Filter::Round {
                precision: args.get("precision").unwrap_or(
                    args.get("_").unwrap_or(&"0")
                ).parse::<u8>().unwrap_or(0),
            },

            // String filters.
            "lowercase" => Filter::Text { case: TextCase::Lower },
            "uppercase" => Filter::Text { case: TextCase::Upper },
            "markdown" => Filter::Markdown,
            "replace" => Filter::Replace {
                find: args.get("find").unwrap_or(
                    args.get("_").unwrap_or(&"")
                ).to_string(),
                replacement: args.get("replacement").unwrap_or(&"").to_string(),
                limit: args.get("limit").map(|s| s.parse::<u8>().ok()).unwrap_or(None),
            },
            "reverse" => Filter::Reverse,
            "truncate" => Filter::Truncate {
                // Attempt to get the characters, but if we can't then we use
                // the unnamed value, defined as "_".
                characters: args.get("characters").unwrap_or(
                    args.get("_").unwrap_or(&"100")
                ).parse::<u8>().unwrap_or(100),
                trail: args.get("trail").unwrap_or(&"...").to_string(),
            },
            "text" => Filter::Text {
                // Default is `case: TextCase::Lower`.
                case: args.get("case").unwrap_or(
                    args.get("_").unwrap_or(&"lower")
                ).parse::<TextCase>().unwrap_or(TextCase::Lower)
            },
            _ => {
                dbg!(name);
                unreachable!();
            }
        })
    })
}

/// Parsers a pipe (`|`) separated list of [`Filter`]s.
///
/// # Examples
/// A single filter.
/// ```rust
/// use blogs_md_easy::{parse_filters, Filter, Span, TextCase};
///
/// // As in {{ £my_variable | lowercase }}
/// let input = Span::new("| lowercase");
/// let (_, filters) = parse_filters(input).unwrap();
/// assert!(matches!(filters[0], Filter::Text { case: TextCase::Lower }));
/// ```
///
/// Multiple filters chained together with `|`.
/// ```rust
/// use blogs_md_easy::{parse_filters, Filter, Span, TextCase};
///
/// // As in {{ £my_variable | lowercase | truncate = trail: ..! }}
/// let input = Span::new("| lowercase | truncate = trail: ..!");
/// let (_, filters) = parse_filters(input).unwrap();
/// assert!(matches!(filters[0], Filter::Text { case: TextCase::Lower }));
/// assert!(matches!(filters[1], Filter::Truncate { .. }));
/// assert_eq!(filters[1], Filter::Truncate {
///     characters: 100,
///     trail: "..!".to_string(),
/// });
/// ```
pub fn parse_filters(input: Span) -> IResult<Span, Vec<Filter>> {
    preceded(
        tuple((space0, tag("|"), space0)),
        separated_list1(tuple((space0, tag("|"), space0)), parse_filter)
    )(input)
}

/// Parse a template [`Placeholder`].
///
/// This is a variable name, surrounded by `{{` and `}}`.  \
/// Whitespace is optional.
///
/// # Examples
/// A simple [`Placeholder`].
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
/// A [`Placeholder`] without whitespace.
/// ```rust
/// use blogs_md_easy::{parse_placeholder, Span};
///
/// let input = Span::new("{{£variable}}");
/// let (_, placeholder) = parse_placeholder(input).unwrap();
/// assert_eq!(placeholder.name.as_str(), "variable");
/// assert_eq!(placeholder.selection.start.offset, 0);
/// assert_eq!(placeholder.selection.end.offset, 14);
/// ```
///
/// A [`Placeholder`] with a single [`Filter`].
/// ```rust
/// use blogs_md_easy::{parse_placeholder, Filter, Span, TextCase};
///
/// let input = Span::new("{{ £variable | uppercase }}");
/// let (_, placeholder) = parse_placeholder(input).unwrap();
/// assert_eq!(placeholder.name.as_str(), "variable");
/// assert_eq!(placeholder.selection.start.offset, 0);
/// assert_eq!(placeholder.selection.end.offset, 28);
/// assert!(matches!(placeholder.filters[0], Filter::Text { case: TextCase::Upper }));
/// ```
///
/// A [`Placeholder`] with a two [`Filter`]s.
/// ```rust
/// use blogs_md_easy::{parse_placeholder, Filter, Span, TextCase};
///
/// let input = Span::new("{{ £variable | lowercase | truncate = characters: 42 }}");
/// let (_, placeholder) = parse_placeholder(input).unwrap();
/// assert_eq!(placeholder.name.as_str(), "variable");
/// assert_eq!(placeholder.selection.start.offset, 0);
/// assert_eq!(placeholder.selection.end.offset, 56);
/// assert!(matches!(placeholder.filters[0], Filter::Text { case: TextCase::Lower }));
/// assert_eq!(placeholder.filters[1], Filter::Truncate { characters: 42, trail: "...".to_string() });
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
/// the first matched placeholder, returning a [`Placeholder`] struct.
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
/// [`Placeholder`].
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
pub fn parse_placeholder_locations(input: Span) -> Result<Vec<Placeholder>, Box<dyn Error>> {
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
/// Convert the meta_values into a [`HashMap`], then parse the title and content
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
pub fn create_variables(markdown: Span, meta_values: Vec<Meta>) -> Result<HashMap<String, String>, Box<dyn Error>> {
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
            return Err("Missing title".to_string())?;
        }
    }
    if !variables.contains_key("content") {
        let content = markdown.fragment().trim().to_string();
        variables.insert("content".to_string(), content);
    }

    Ok(variables)
}

/// Make the start of each word capital, splitting on `sep`.
///
/// # Examples
/// A simple phrase with a space.
/// ```rust
/// use blogs_md_easy::split_string;
///
/// let phrase = "Hello World";
/// let phrase = split_string(phrase.to_string(), &[' ', '-']);
/// //let phrase = phrase.iter().map(|word| word.as_str()).collect::<&str>();
/// assert_eq!(phrase, vec!["Hello", " ", "World"]);
/// ```
///
/// A name with a hyphen.
/// ```rust
/// use blogs_md_easy::split_string;
///
/// let phrase = "John Doe-Bloggs";
/// let phrase = split_string(phrase.to_string(), &[' ', '-']);
/// //let phrase = phrase.iter().map(|word| word.as_str()).collect::<&str>();
/// assert_eq!(phrase, vec!["John", " ", "Doe", "-", "Bloggs"]);
/// ```
///
/// Two separators in a row.
/// ```rust
/// use blogs_md_easy::split_string;
///
/// let phrase = "Hello, World!";
/// let phrase = split_string(phrase.to_string(), &[' ', ',', '!']);
/// //let phrase = phrase.iter().map(|word| word.as_str()).collect::<&str>()
/// assert_eq!(phrase, vec!["Hello", ",", " ", "World", "!"]);
/// ```
pub fn split_string(phrase: String, separators: &[char]) -> Vec<String> {
    let mut words = Vec::new();
    let mut current_word = String::new();

    for c in phrase.chars() {
        // If we hit a separator; push the current word, then the separator.
        // Otherwise, add the character to the current word.
        if separators.contains(&c) {
            // Make sure that we aren't pushing an empty string into the Vec.
            // This cannot be added as an `&&` above, because otherwise it
            // pushes a separator onto the start of `current_word` in the event
            // that we have two separators in a row.
            if !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
            words.push(c.to_string());
        } else {
            current_word.push(c);
        }
    }

    if !current_word.is_empty() {
        words.push(current_word);
    }
    words
}

/// Take a variable, and run it through a [`Filter`] function to get the new
/// output.
///
/// For an example of how these [`Filter`]s work within a [`Placeholder`], see
/// [`parse_placeholder`].
///
/// # Examples
/// [`Filter`] that has no arguments.
/// ```rust
/// use blogs_md_easy::{render_filter, Filter, TextCase};
///
/// let variable = "hello, world!".to_string();
/// assert_eq!("HELLO, WORLD!", render_filter(variable, &Filter::Text { case: TextCase::Upper }));
/// ```
///
/// [`Filter`] that has arguments.
/// ```rust
/// use blogs_md_easy::{render_filter, Filter};
///
/// let variable = "hello, world!".to_string();
/// assert_eq!("hello...", render_filter(variable, &Filter::Truncate { characters: 5, trail: "...".to_string() }));
/// ```
pub fn render_filter(variable: String, filter: &Filter) -> String {
    match filter {
        // Maths filters.
        Filter::Ceil => variable.parse::<f64>().unwrap_or_default().ceil().to_string(),
        Filter::Floor => variable.parse::<f64>().unwrap_or_default().floor().to_string(),
        Filter::Round { precision } => variable
            .parse::<f64>()
            .unwrap_or_default()
            // Be default, Rust rounds away all decimals.
            // So we want to move the decimal places `precision` places to the
            // left.
            .mul(10_f64.powi((*precision as u32) as i32))
            // Now round, removing all decimal places.
            .round()
            // Now move the decimal place back.
            .div(10_f64.powi((*precision as u32) as i32))
            .to_string(),

        // String filters.
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
        Filter::Replace { find, replacement, limit } => {
            if limit.is_none() {
                variable.replace(find, replacement)
            } else {
                // Subtract 1 to account for the final iteration.
                let segments = variable.split(find).count() - 1;

                variable
                .split(find)
                .enumerate()
                .map(|(count, part)| {
                    // We can safely unwrap, because `limit.is_some()`.
                    if (count as u8) < limit.unwrap() {
                        format!("{}{}", part, replacement)
                    } else {
                        format!("{}{}", part, if count < segments { find } else { "" })
                    }
                })
                .collect::<Vec<String>>()
                .join("")
            }
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
        Filter::Text { case } => {
            let separators = &[' ', ',', '!', '-', '_'];
            match case {
                TextCase::Lower => variable.to_lowercase(),
                TextCase::Upper => variable.to_uppercase(),
                TextCase::Title => {
                    split_string(variable, separators)
                    .into_iter()
                    .map(|word| {
                        if word.len() == 1 && separators.contains(&word.chars().next().unwrap_or_default()) {
                            word
                        } else {
                            word[0..1].to_uppercase() + &word[1..]
                        }
                    })
                    .collect::<String>()
                },
                TextCase::Kebab => variable
                    .to_lowercase()
                    .split(|c| separators.contains(&c))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<&str>>()
                    .join("-"),
                TextCase::Snake => variable
                    .to_lowercase()
                    .split(|c| separators.contains(&c))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<&str>>()
                    .join("_"),
                TextCase::Pascal => variable
                    .split(|c| separators.contains(&c))
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        let mut c = s.chars();
                        match c.next() {
                            Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(""),
                TextCase::Camel => variable
                    .split(|c| separators.contains(&c))
                    .filter(|s| !s.is_empty())
                    .enumerate()
                    .map(|(i, s)| {
                        let mut c = s.chars();
                        match c.next() {
                            Some(first) => (if i == 0 {
                                first.to_lowercase().collect::<String>()
                            } else {
                                first.to_uppercase().collect::<String>()
                            }) + c.as_str(),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(""),
                TextCase::Invert => variable.chars().fold(String::new(), |mut str, c| {
                    if c.is_lowercase() {
                        str.push_str(&c.to_uppercase().collect::<String>());
                    } else {
                        str.push_str(&c.to_lowercase().collect::<String>());
                    }
                    str
                }),
            }
        },
    }
}
