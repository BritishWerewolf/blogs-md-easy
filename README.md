[![Crates.io Version](https://img.shields.io/crates/v/blogs-md-easy)](https://crates.io/crates/blogs-md-easy)
[![GitHub Repo stars](https://img.shields.io/github/stars/BritishWerewolf/blogs-md-easy)](https://github.com/BritishWerewolf/blogs-md-easy)

# Blogs Made Easy
Iteratively convert a collection of Markdown files into a respective HTML template.

## Installation
Ensure that you have [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.  
Then you can run the following command in your terminal.
```sh
$ cargo install blogs-md-easy
```

## Usage
Below is the help page for the program.
```
Iteratively convert a collection of Markdown files into a respective HTML template.

Usage: blogs-md-easy.exe [OPTIONS] --template <FILE> --markdowns <FILES>...

Options:
  -t, --template <FILE>       HTML template that the Markdowns will populate
  -m, --markdowns <FILES>...  List of Markdown files ending in .md
  -o, --output-dir <DIR>      Output directory, defaults to the Markdown's directory
  -h, --help                  Print help
  -V, --version               Print version
```

### Template
Templates are `.html` files that use variables to populate the file.

Variables must follow these rules:
* Must be wrapped in `{{` and `}}`, white space either side is optional.
* Must be prefixed with a `£` character.
* Must start with a letter from a to z, case insensitive.
* Must only contain the following characters: `a-z`, `0-9`, `_`.

Two variables are required: `title` and `content`.  
More on how these variables are parsed in the below section.

Example of a valid template page.
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="{{ £description }}">

    <title>{{ £title }}</title>
</head>
<body>
    <section>
        <h1>{{ £title }}</h1>
        <p>Authored by {{ £author }}</p>
    </section>
    <section>{{ £content }}</section>
</body>
</html>
```
Firstly, there is a `£description` variable that is used for the `meta` description tag.  
This will be provided within the `meta` section of each Markdown file, as will the `£author` variable.

Additionally, the `£title` variable is used in two locations: for the document title, and as a heading.  
Variables can be reused as many times as required, and will be replaced, providing they follow the above rules.

Finally, the `£content` variable is automatically generated based on the entire body of the Markdown file.

#### Directives
It's possible to mutate the placeholders during rendering by providing directives.  
A directive is just a way of applying a pre-defined function to any placeholder variable.

Let's use our previous template, and apply a simple directive to the `£title` variable.
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="{{ £description }}">

    <title>{{ £title }}</title>
</head>
<body>
    <section>
        <h1>{{ £title | uppercase }}</h1>
        <p>Authored by {{ £author }}</p>
    </section>
    <section>{{ £content }}</section>
</body>
</html>
```
By providing the function after a pipe (`|`) character, we can mutate that variable in that particular location. This is particularly useful in cases where a placeholder is required multiple times through a template, but the formatting should be different in all cases.

There are currently three supported directives:
* `lowercase` - Convert the value to lowercase.
* `uppercase` - Convert the value to uppercase.
* `markdown` - Convert the value from Markdown into HTML.

By default, no directives will be provided, unless specified within the template, with the exception of `£content` which will have `markdown` applied.

Directives are case insensitive, meaning `| uppercase` is the same as `| UPPERCASE`. They can also be chained together, such as in the following example.
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="{{ £description }}">

    <title>{{ £title }}</title>
</head>
<body>
    <section>
        {{ £title | uppercase | markdown }}
        <p>Authored by {{ £author }}</p>
    </section>
    <section>{{ £content }}</section>
</body>
</html>
```
Here we are converting the title to uppercase, and then mutating the value into markdown.  
Chained directives are evaluated from left to right.

### Markdowns
[Markdowns](https://daringfireball.net/projects/markdown) are simple text files that contain any text, and an optional `meta` section.

The `meta` section of the Markdown file is unique to this program.  
This section must be at the top of the document, and will be start with either `:meta` or `<meta>`, and closed with `:meta` or `</meta>`.

Use the `meta` section to provide the template values for the variables that have been defined. As a result, the variables used in the `meta` section must adhere to the rules that apply to the template variables.

A warning will be generated if a variable is declared in the Markdown, but not used.  
Conversely, an error will cause the execution of the program to stop if the template doesn't receive values for all variables.

There is no requirement to declare the `meta` section; however if you do not provide an `<h1>` (the Markdown `#` is acceptable too) at the top of your content, then a title variable is required.

Example of a Markdown file, where the title is parsed from the document.
```md
:meta
author = John Doe
description = This will appear in Search Engines.
:meta
# Markdown Title
This is the content of our file.
```
Using the above template, would generate the following output.
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="This will appear in Search Engines.">

    <title>Markdown Title</title>
</head>
<body>
    <section>
        <h1>Markdown Title</h1>
        <p>Authored by John Doe</p>
    </section>
    <section>
        <h1>Markdown Title</h1>
        <p>This is the content of our file.</p>
    </section>
</body>
</html>
```

Another example where we overwrite the title variable and use the prefix.
```md
<meta>
£author = John Doe
£title = Meta Title
£description = This will appear in Search Engines.
</meta>
# Markdown Title
This is the content of our file.
```

Which would generate the following output
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="This will appear in Search Engines.">

    <title>Meta Title</title>
</head>
<body>
    <section>
        <h1>Meta Title</h1>
        <p>Authored by John Doe</p>
    </section>
    <section>
        <h1>Markdown Title</h1>
        <p>This is the content of our file.</p>
    </section>
</body>
</html>
```

#### Comments
It's possible to add comments to the meta section, by starting a line with either `#` or `//`.  
Comments will be parsed and the leading comment prefix will be removed, however this is superfluous as they will be replaced with None during parsing, and susequently removed.
```md
:meta
// This is a comment.
author = John Doe
# This is another type of comment
description = This will appear in Search Engines.
:meta
```
The above meta key-values that would be parsed would be `author` and `description`, with the values being `John Doe` and `This will appear in Search Engines.` respectively.

### Output
All HTML files will be generated with the exact same name as the Markdown that they are converting, but with the `.html` extension.

By default, the file will be created in the same directory as the Markdown file, however, by providing `--output-dir` (or `-o` if that's easier) the output directory can be changed.  
This will not rename the file, but rather just place it in the specified directory.

Some formatting will be applied to the generated output, but it will likely need human intervention if you want the document to be formatted correct - such as indenting.  
Currently, a new line is placed before all headings (from `h2` to to `h6`), but nothing else is changed.

## Todo List
- [ ] Add if statements to render content based on a condition.
- [x] Add directives to placeholders.
    - [ ] Add directives that support arguments.
- [ ] Add tag filter to prevent parsing scripts.
- [ ] Add better handling for errors in meta sections.
    - For example if a key is passed without a value, then no meta values are parsed.
- [ ] Ability to truncate to either characters or tags.
    - Useful for creating to short templates.
- [ ] Generate for multiple templates at once.
- [ ] Option to run against markdowns to determine if they were built off a different template.
- [x] Add comments to the meta section.
- [ ] Add mutliline values to the meta section.
- [ ] Ensure meta tags must be the same.
    - If `:meta` is used to start the section, `:meta` should close it. Vice versa with `<meta>` and `</meta>`.
- [ ] Formatting of the generated file.
- [x] Document all functions.
- [ ] Reduce complexity of `main` function, to make it easier to test.
