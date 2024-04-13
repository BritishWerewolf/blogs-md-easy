use anyhow::anyhow;
use blogs_md_easy::{create_variables, parse_meta_section, parse_placeholder_locations, render_filter, replace_substring, Span};
use clap::Parser;
use std::{collections::HashMap, ffi::OsStr, fs, path::PathBuf};

////////////////////////////////////////////////////////////////////////////////
// Structs and types
/// A list of the possible features that can be allowed.
#[derive(Debug, PartialEq, Eq)]
enum AllowList {
    /// Allows anything that is unused to be acceptable.
    Unused,
    /// Allows any variable declared within the `meta` section.
    ///
    /// With this enabled, variables that are declared in the Markdown, but not
    /// used within the Template, will not display a warning in the console.
    UnusedVariables,
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// HTML template that the Markdowns will populate.
    #[arg(short, long, required = true, alias = "template", value_name = "FILES", num_args = 1..)]
    templates: Vec<PathBuf>,

    // num_args is required so that we don't have to specify the option before
    // each file...
    // `-m file.md file2.md`    rather than    `-m file.md -m file2.md`
    /// List of Markdown files ending in .md.
    #[arg(short, long, required = true, value_name = "FILES", num_args = 1..)]
    markdowns: Vec<PathBuf>,

    /// Output directory, defaults to the Markdown's directory.
    #[arg(short, long, value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Define an allow list for features.
    #[arg(short, long, value_name = "RULES", num_args = 1..)]
    allow: Vec<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let templates = cli.templates;
    let allow_list: Vec<AllowList> = cli.allow.into_iter().filter_map(|list| {
        match list.trim().to_lowercase().as_str() {
            "unused" => Some(AllowList::Unused),
            "unused-variables" | "unused_variables" => Some(AllowList::UnusedVariables),
            _ => None,
        }
    }).collect();

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

    for template_path in &templates {
        // Check that the actual template exists.
        if !template_path.try_exists().map_err(|_| anyhow!("The template could not be found."))? {
            Err(anyhow!("The template file does not exist."))?;
        };
        let template = std::fs::read_to_string(&template_path)?;
        let template = Span::new(&template);

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
            if !allow_list.contains(&AllowList::Unused) && !allow_list.contains(&AllowList::UnusedVariables) {
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

            // Get the template extension, because the user might be passing in
            // something like an SVG.
            let template_ext = template_path.extension().unwrap_or(OsStr::new("html"));

            // Get the output path where the `.md` is replaced with `.html`.
            let mut output_path = match cli.output_dir.clone() {
                Some(path) => path.join(markdown_url.with_extension(template_ext).file_name().unwrap()),
                None => markdown_url.with_extension(template_ext),
            };

            // If there are multiple templates, then add that to the output path
            // to avoid overwriting issues.
            if templates.len() > 1 {
                output_path = output_path.with_file_name(format!(
                    "{}-{}",
                    &template_path.file_stem().unwrap_or_default().to_str().unwrap_or_default(),
                    output_path.file_stem().unwrap_or_default().to_str().unwrap_or_default()
                )).with_extension("html");
            }

            // Create all folders from the path.
            if let Some(path) = output_path.parent() {
                if !path.exists() {
                    fs::create_dir_all(path)?;
                }
            }

            fs::write(output_path, html_doc)?;
        }
    }

    Ok(())
}
