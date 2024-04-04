use std::collections::HashMap;

use blogs_md_easy::{create_variables, parse_meta_comment, parse_meta_key_value, parse_meta_section, parse_placeholder, parse_placeholder_directive_enum, parse_placeholder_locations, parse_title, parse_until_eol, parse_variable, render_directive, replace_substring, Directive, Marker, Meta, Selection, Span};
use nom::combinator::opt;

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
fn cannot_parse_mismatch_meta_tags() {
    let input = Span::new(":meta\nauthor = John Doe\n</meta>");
    let meta_values = parse_meta_section(input);
    assert!(meta_values.is_err());
    assert_eq!(input.fragment(), &":meta\nauthor = John Doe\n</meta>");
}

#[test]
fn can_parse_meta_section_with_comments() {
    let input = Span::new(":meta\n// This is an author\nauthor = John Doe\n# This is the publish date\npublish_date = 2024-01-01\n:meta\n# Markdown title\nThis is my content");
    let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

    // We get None, Some, None, Some.
    assert!(meta.len() == 4);
    // Then filter and unwrap.
    let meta: Vec<Meta> = meta.into_iter().flatten().collect();

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
fn can_render_uppercase_directive() {
    let variable = "hello, world!".to_string();
    let variable = render_directive(variable, &Directive::Uppercase);
    assert_eq!("HELLO, WORLD!", variable);
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
fn can_render_lowercase_directive() {
    let variable = "HELLO, WORLD!".to_string();
    let variable = render_directive(variable, &Directive::Lowercase);
    assert_eq!("hello, world!", variable);
}

#[test]
fn can_render_markdown_directive() {
    let variable = "# Title\nParagraph\n\n[link](https://example.com)".to_string();
    let variable = render_directive(variable, &Directive::Markdown);
    assert_eq!("<h1>Title</h1>\n<p>Paragraph</p>\n<p><a href=\"https://example.com\">link</a></p>", variable);
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
    assert_eq!(directive, Directive::Date {
        format: "dd-mmm-yyyy".to_string()
    });
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

    assert_eq!(placeholders.iter().find(|p| &p.name == "content").expect("content to exist").selection, Selection {
        start: Marker { line: 8, offset: 123 },
        end: Marker { line: 8, offset: 138 },
    });

    assert_eq!(placeholders.iter().find(|p| &p.name == "author").expect("author to exist").selection, Selection {
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
            // Used to deref the variable.
            let mut variable = variable.to_owned();

            for directive in &placeholder.directives {
                variable = render_directive(variable, directive);
            }

            html_doc = replace_substring(&html_doc, placeholder.selection.start.offset, placeholder.selection.end.offset, &variable);
        } else {
            assert!(variables.contains_key(&placeholder.name));
        }
    }

    assert_eq!(html_doc, "<html>\n<head>\n<title>Meta title</title>\n</head>\n<body>\n<h1>Meta title</h1>\n<small>By John Doe</small>\n<section><h1>Markdown title</h1>\n<p>This is my content</p></section>\n</body>\n</html>");
}
