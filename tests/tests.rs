use std::collections::HashMap;

use blogs_md_easy::{create_variables, parse_filter, parse_filter_args, parse_filter_key_value, parse_filters, parse_meta_comment, parse_meta_key_value, parse_meta_section, parse_placeholder, parse_placeholder_locations, parse_title, parse_until_eol, parse_variable, render_filter, replace_substring, Filter, Marker, Meta, Selection, Span};
use nom::combinator::opt;

////////////////////////////////////////////////////////////////////////////////
// Parsers, variables, and placeholders

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

////////////////////////////////////////////////////////////////////////////////
// Meta Section

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
        Meta::new("title", "Meta title"),
        Meta::new("author", "John Doe"),
    ]);

    assert_eq!(input.fragment(), &"# Markdown title\nThis is my content");
}

#[test]
fn can_parse_metadata_tag() {
    let input = Span::new("<meta>\ntitle = Meta title\nauthor = John Doe\n</meta>\n# Markdown title\nThis is my content");
    let (input, meta) = parse_meta_section(input).expect("to parse the meta values");

    assert_eq!(meta, vec![
        Meta::new("title", "Meta title"),
        Meta::new("author", "John Doe"),
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

    assert!(meta.len() == 2);

    assert_eq!(meta, vec![
        Meta::new("author", "John Doe"),
        Meta::new("publish_date", "2024-01-01"),
    ]);

    assert_eq!(input.fragment(), &"# Markdown title\nThis is my content");
}

////////////////////////////////////////////////////////////////////////////////
// Placeholders

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
fn can_parse_placeholder_with_no_filter() {
    // Filters are case insensitive.
    let input = Span::new("<p>{{ £variable }}</p>");
    let placeholders = parse_placeholder_locations(input).expect("to parse placeholder");

    assert_eq!(placeholders.len(), 1);
    assert_eq!(placeholders[0].name, "variable".to_string());
    assert_eq!(placeholders[0].selection.start.offset, 3);
    assert_eq!(placeholders[0].selection.end.offset, 19);
    assert_eq!(placeholders[0].filters, vec![]);
}

#[test]
fn can_parse_placeholder_uppercase_filter() {
    // Filters are case insensitive.
    let input = Span::new("<p>{{ £variable | UPPERCASE }}</p>");
    let placeholders = parse_placeholder_locations(input).expect("to parse placeholder");

    assert_eq!(placeholders.len(), 1);
    assert_eq!(placeholders[0].name, "variable".to_string());
    assert_eq!(placeholders[0].selection.start.offset, 3);
    assert_eq!(placeholders[0].selection.end.offset, 31);
    assert_eq!(placeholders[0].filters, vec![Filter::Uppercase]);
}

////////////////////////////////////////////////////////////////////////////////
// Placeholders with filters

#[test]
fn can_parse_placeholder_with_filter_in_uppercase() {
    let input = Span::new("<p>{{ £variable | UPPERCASE }}</p>");
    let placeholders = parse_placeholder_locations(input).expect("to parse placeholder");

    assert_eq!(placeholders.len(), 1);
    assert_eq!(placeholders[0].name, "variable".to_string());
    assert_eq!(placeholders[0].selection.start.offset, 3);
    assert_eq!(placeholders[0].selection.end.offset, 31);
    assert_eq!(placeholders[0].filters, vec![Filter::Uppercase]);
}

#[test]
fn can_parse_placeholder_with_filter_in_lowercase() {
    let input = Span::new("<p>{{ £variable | lowercase }}</p>");
    let placeholders = parse_placeholder_locations(input).expect("to parse placeholder");

    assert_eq!(placeholders.len(), 1);
    assert_eq!(placeholders[0].name, "variable".to_string());
    assert_eq!(placeholders[0].selection.start.offset, 3);
    assert_eq!(placeholders[0].selection.end.offset, 31);
    assert_eq!(placeholders[0].filters, vec![Filter::Lowercase]);
}

#[test]
fn can_parse_two_placeholder_filters() {
    let input = Span::new("<p>{{ £title | uppercase | lowercase }}</p>");
    let placeholders = parse_placeholder_locations(input).expect("parse placeholders");

    assert_eq!(placeholders.len(), 1);
    assert_eq!(placeholders[0].name, "title".to_string());
    assert_eq!(placeholders[0].selection.start.offset, 3);
    assert_eq!(placeholders[0].selection.end.offset, 40);
    assert_eq!(placeholders[0].filters, vec![Filter::Uppercase, Filter::Lowercase]);
}

////////////////////////////////////////////////////////////////////////////////
// Filters

#[test]
fn can_parse_filter_arg_value() {
    let input = Span::new("characters: 20");
    let (input, (arg, value)) = parse_filter_key_value(input).expect("parse key-value");

    assert_eq!(input.fragment(), &"");
    assert_eq!(arg, "characters");
    assert_eq!(value, "20");
}

#[test]
fn can_parse_filter_arg_value_vec() {
    let input = Span::new("characters: 20, trail: ...");
    let (input, args) = parse_filter_args(input).expect("parse args");

    assert_eq!(input.fragment(), &"");
    assert_eq!(args, vec![
        ("characters", "20"),
        ("trail", "...")
    ]);
}

#[test]
fn can_parse_filter_with_no_args() {
    let input = Span::new("lowercase");
    let (input, filter) = parse_filter(input).expect("parse filter");

    assert_eq!(input.fragment(), &"");
    assert!(matches!(filter, Filter::Lowercase));
}

#[test]
fn can_parse_filter_with_args() {
    let input = Span::new("truncate = characters: 15, trail: ...");
    let (input, filter) = parse_filter(input).expect("parse filter");

    assert_eq!(input.fragment(), &"");
    assert!(matches!(filter, Filter::Truncate { .. }));

    if let Filter::Truncate { characters, trail } = filter {
        assert_eq!(characters, 15);
        assert_eq!(trail, "...");
    }
}

#[test]
fn can_parse_filter_with_defaults() {
    let input = Span::new("truncate = trail: ...");
    let (input, filter) = parse_filter(input).expect("parse filter");

    assert_eq!(input.fragment(), &"");
    assert!(matches!(filter, Filter::Truncate { .. }));

    if let Filter::Truncate { characters, trail } = filter {
        assert_eq!(characters, 100);
        assert_eq!(trail, "...");
    }
}

#[test]
fn can_parse_filter_with_just_value() {
    let input = Span::new("truncate = 15");
    let (input, filter) = parse_filter(input).expect("parse filter");

    assert_eq!(input.fragment(), &"");
    assert!(matches!(filter, Filter::Truncate { .. }));

    if let Filter::Truncate { characters, trail } = filter {
        assert_eq!(characters, 15);
        assert_eq!(trail, "...");
    }
}

#[test]
fn can_parse_filter_with_args_not_provided() {
    let input = Span::new("truncate");
    let (input, filter) = parse_filter(input).expect("parse filter");

    assert_eq!(input.fragment(), &"");
    assert!(matches!(filter, Filter::Truncate { .. }));

    if let Filter::Truncate { characters, trail } = filter {
        assert_eq!(characters, 100);
        assert_eq!(trail, "...");
    }
}

#[test]
fn can_parse_two_filters() {
    let input = Span::new("| truncate = characters: 20 | lowercase");
    let (input, filters) = parse_filters(input).expect("parse filters");

    assert_eq!(input.fragment(), &"");
    assert_eq!(filters.len(), 2);
    dbg!(&filters);

    assert!(matches!(filters[0], Filter::Truncate { .. }));
    assert!(matches!(filters[1], Filter::Lowercase));

    if let Filter::Truncate { characters, trail } = &filters[0] {
        assert_eq!(characters, &20);
        assert_eq!(trail, "...");
    }
}

#[test]
fn can_parse_all_filters() {
    // We need this test that we don't forget to create match the string to the
    // filter.
    let filters: Vec<(Filter, Filter)> = vec![
        (Filter::Lowercase, parse_filter(Span::new("lowercase")).expect("lowercase").1),
        (Filter::Uppercase, parse_filter(Span::new("uppercase")).expect("uppercase").1),
        (Filter::Markdown, parse_filter(Span::new("markdown")).expect("markdown").1),
        (Filter::Reverse, parse_filter(Span::new("reverse")).expect("reverse").1),
        (Filter::Truncate { characters: 100, trail: "...".to_string() }, parse_filter(Span::new("truncate")).expect("truncate").1),
    ];

    // Maybe a bit verbose, but this ensures that the compiler will catch new
    // filters immediately.
    for (expected_filter, actual_filter) in filters {
        match actual_filter {
            Filter::Lowercase => assert_eq!(expected_filter, Filter::Lowercase),
            Filter::Uppercase => assert_eq!(expected_filter, Filter::Uppercase),
            Filter::Markdown => assert_eq!(expected_filter, Filter::Markdown),
            Filter::Reverse => assert_eq!(expected_filter, Filter::Reverse),
            Filter::Truncate { characters, trail } => {
                assert_eq!(expected_filter, Filter::Truncate { characters, trail });
            }
        }
    }
}

#[test]
fn filter_lowercase_works() {
    let input = "HELLO, WORLD!".to_string();
    let output = render_filter(input, &Filter::Lowercase);
    assert_eq!(output, "hello, world!");
}

#[test]
fn filter_uppercase_works() {
    let input = "hello, world!".to_string();
    let output = render_filter(input, &Filter::Uppercase);
    assert_eq!(output, "HELLO, WORLD!");
}

#[test]
fn filter_markdown_works() {
    let input = "# Title\nFirst _paragraph_.  \nNewline.\n\nSecond paragraph with [link](https://example.com).\n\n* Unordered list.\n\n1. Ordered list.".to_string();
    let output = render_filter(input, &Filter::Markdown);
    assert_eq!(output, "<h1>Title</h1>\n<p>First <em>paragraph</em>.<br />\nNewline.</p>\n<p>Second paragraph with <a href=\"https://example.com\">link</a>.</p>\n<ul>\n<li>Unordered list.</li>\n</ul>\n<ol>\n<li>Ordered list.</li>\n</ol>");
}

#[test]
fn filter_reverse_works() {
    let input = "Hello, World!".to_string();
    let output = render_filter(input, &Filter::Reverse);
    assert_eq!(output, "!dlroW ,olleH");
}

#[test]
fn filter_truncate_works() {
    let input = "Hello, World!".to_string();
    let output = render_filter(input, &Filter::Truncate { characters: 7, trail: "--".to_string() });
    assert_eq!(output, "Hello, --");
}

#[test]
fn can_parse_truncate_filter() {
    // Providing both arguments.
    let input = Span::new("| truncate = characters: 7, trail: --");
    let (_, filters) = parse_filters(input).expect("parse both arguments");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], Filter::Truncate { characters: 7, trail: "--".to_string() });

    // Providing just characters.
    let input = Span::new("| truncate = characters: 7");
    let (_, filters) = parse_filters(input).expect("parse just characters");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], Filter::Truncate { characters: 7, trail: "...".to_string() });

    // Providing just trail.
    let input = Span::new("| truncate = trail: --");
    let (_, filters) = parse_filters(input).expect("parse just trail");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], Filter::Truncate { characters: 100, trail: "--".to_string() });

    // Providing just default value.
    let input = Span::new("| truncate = 42");
    let (_, filters) = parse_filters(input).expect("parse default value");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], Filter::Truncate { characters: 42, trail: "...".to_string() });

    // Providing no arguments.
    let input = Span::new("| truncate");
    let (_, filters) = parse_filters(input).expect("parse no arguments");
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], Filter::Truncate { characters: 100, trail: "...".to_string() });
}

#[test]
fn can_render_truncate_filter() {
    // Providing both arguments.
    let input = Span::new("{{ £title | truncate = characters: 7, trail: -- }}");
    let (_, placeholder) = parse_placeholder(input).expect("to parse placeholder");
    let title = "Hello, World!".to_string();
    assert_eq!(render_filter(title, &placeholder.filters[0]), "Hello, --".to_string());

    // Providing just characters.
    let input = Span::new("{{ £title | truncate = characters: 7 }}");
    let (_, placeholder) = parse_placeholder(input).expect("to parse placeholder");
    let title = "Hello, World!".to_string();
    assert_eq!(render_filter(title, &placeholder.filters[0]), "Hello, ...".to_string());

    // Providing just trail.
    let input = Span::new("{{ £title | truncate = trail: -- }}");
    let (_, placeholder) = parse_placeholder(input).expect("to parse placeholder");
    let title = "Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World!".to_string();
    assert_eq!(render_filter(title, &placeholder.filters[0]), "Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! He--".to_string());

    // Providing just trail on a short string (no trail added).
    let input = Span::new("{{ £title | truncate = trail: -- }}");
    let (_, placeholder) = parse_placeholder(input).expect("to parse placeholder");
    let title = "Hello, World!".to_string();
    assert_eq!(render_filter(title, &placeholder.filters[0]), "Hello, World!".to_string());

    // Providing just default argument.
    let input = Span::new("{{ £title | truncate = 8 }}");
    let (_, placeholder) = parse_placeholder(input).expect("to parse placeholder");
    let title = "Hello, World! Hello, World!".to_string();
    assert_eq!(render_filter(title, &placeholder.filters[0]), "Hello, W...".to_string());

    // Providing no arguments.
    let input = Span::new("{{ £title | truncate }}");
    let (_, placeholder) = parse_placeholder(input).expect("to parse placeholder");
    let title = "Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World!".to_string();
    assert_eq!(render_filter(title, &placeholder.filters[0]), "Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! Hello, World! He...".to_string());
}

////////////////////////////////////////////////////////////////////////////////
// Integration tests

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
        Meta::new("title", "Meta title"),
        Meta::new("author", "John Doe"),
    ]);
    let variables: HashMap<String, String> = create_variables(markdown, meta_values).expect("to create variables");

    let mut html_doc = template.to_string();
    for placeholder in &placeholders {
        if let Some(variable) = variables.get(&placeholder.name) {
            // Used to deref the variable.
            let mut variable = variable.to_owned();

            for filter in &placeholder.filters {
                variable = render_filter(variable, filter);
            }

            html_doc = replace_substring(&html_doc, placeholder.selection.start.offset, placeholder.selection.end.offset, &variable);
        } else {
            assert!(variables.contains_key(&placeholder.name));
        }
    }

    assert_eq!(html_doc, "<html>\n<head>\n<title>Meta title</title>\n</head>\n<body>\n<h1>Meta title</h1>\n<small>By John Doe</small>\n<section><h1>Markdown title</h1>\n<p>This is my content</p></section>\n</body>\n</html>");
}
