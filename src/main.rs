use anyhow::anyhow;
use blogs_md_easy::{create_variables, parse_meta_section, parse_placeholder_locations, render_filter, replace_substring, Span};
use clap::Parser;
use std::{collections::HashMap, fs, path::PathBuf};

////////////////////////////////////////////////////////////////////////////////
// Stucts and types
#[derive(Debug, Parser)]
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
                if unused_variables.len() == 1_usize { "" } else { "s" },
                &markdown_url.to_string_lossy(),
                unused_variables.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", ")
            );
        }

        for placeholder in &placeholders {
            if let Some(variable) = variables.get(&placeholder.name) {
                // Used to deref the variable.
                let mut variable = variable.to_owned();

                for filter in &placeholder.filters {
                    variable = render_filter(variable, filter);
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
