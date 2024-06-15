[![Crates.io Version](https://img.shields.io/crates/v/blogs-md-easy)](https://crates.io/crates/blogs-md-easy)
[![docs.rs tests](https://img.shields.io/docsrs/blogs-md-easy)](https://docs.rs/blogs-md-easy)
[![GitHub Repo stars](https://img.shields.io/github/stars/BritishWerewolf/blogs-md-easy)](https://github.com/BritishWerewolf/blogs-md-easy)

# Blogs Made Easy
Iteratively convert a collection of Markdown files into a respective template.

## Installation
Blogs Made Easy is available as both a binary and a library, both from this package.

In both cases, ensure that you have [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.

### Binary
Open your Terminal, and run the following command.
```sh
$ cargo install blogs-md-easy
```

### Library
Update your `Cargo.toml` by running the following command.
```sh
$ cargo add blogs-md-easy
```

## Usage
Below is the help page for the program binary, if you want to read the documentation for the library, that is available on [docs.rs](https://docs.rs/blogs-md-easy) (and will always have the most up to date documentation).
```
Iteratively convert a collection of Markdown files into a respective template.

Usage: blogs-md-easy.exe [OPTIONS] --templates <FILES>... --markdowns <FILES>...

Options:
  -t, --templates <FILES>...  HTML template that the Markdowns will populate
  -m, --markdowns <FILES>...  List of Markdown files ending in .md
  -o, --output-dir <DIR>      Output directory, defaults to the Markdown's directory
  -a, --allow <RULES>...      Define an allow list for features
  -h, --help                  Print help
  -V, --version               Print version
```

### Templates
Templates can be any file, as long as they are text based. Typically they will be in a format like `.html`, or `.xml`, although there is no restrictions placed here.  
As long as you can write placeholders (more on these below), then any format is acceptable.

If more than a single template is provided, then the file stem will be used in the name of the output file. This is to avoid each template generating a new file and overwriting previous files.  
In the case that a single template is used, the output file name will simple be the Markdown file with the extension of the template instead of `.md`.

Variables must follow these rules:
* Must be wrapped in `{{` and `}}`, white space either side is optional.
* Must be prefixed with a `£` or `$` character.
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
This will be provided within the `meta` section of each Markdown file, as will the `£author` variable. It's important to note that the `meta` section of the Markdown, and the HTML file have no correlation.

Additionally, the `£title` variable is used in two locations: for the document title, and as a heading.  
Variables can be reused as many times as required, and will be replaced, providing they follow the above rules.

Finally, the `£content` variable is automatically generated based on the entire body of the Markdown file.

#### Filters
It's possible to mutate the placeholders during rendering by providing filters.  
A filter is just a way of applying a pre-defined function to any placeholder variable.

Let's use our previous template, and apply a simple filter to the `£title` variable.
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

These are many filters available and they are all documented thoroughly on [docs.rs](https://docs.rs/blogs-md-easy/latest/blogs_md_easy/enum.Filter.html).

In essence, you only need to provide a filter's name - even if that filter accepts arguments.  
If the filter accepts arguments, then one argument will be considered "default" and it's name can be omitted. These defaults are described below, otherwise you must provide the name of the argument.  
All arguments are given a default value too.

We'll talk about arguments later on, but for now, know that the argument name is optional and only a value is required.
* `ceil` - Rounds a numeric value up. Returns `0` for any non numeric value.
* `floor` - Rounds a numeric value down. Returns `0` for any non numeric value.
* `round` - Rounds a numeric value. Returns `0` for any non numeric value.
    * `precision` - **default** - The number of decimal places.
* `lowercase` - Convert the value to lowercase.
* `uppercase` - Convert the value to uppercase.
* `markdown` - Convert the value from Markdown into HTML.
* `replace` - Replace a given string with another string, this can be limited too.
    * `find` - **default** - What we want to `replace`.
    * `replacement` - What we will replace `find` with. Defaults to an empty string.
    * `limit` - How many instances should be replaced from the start of the string. Defaults to `None`, otherwise should be a numeric value.
* `reverse` - Reverse the string order.
* `truncate` - Truncate the value to the given length, and adds trailing character(s) if the string is truncated.
    * `characters` - **default** - The number of characters to limit a string to.
    * `trail` - The character(s) to add to the end of the string if it is truncated.

By default, no filters will be provided, unless specified within the template, with the exception of `£content` which will have `markdown` applied.

Filters are case insensitive, meaning `| uppercase` is the same as `| UPPERCASE`. They can also be chained together, such as in the following example.
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
Chained filters are evaluated from left to right.

#### Filters with arguments
For some filters, there are optional arguments that can be provided. This is done through a comma separated list after an equals (`=`) sign.  
Every filter can be provided with just the name, as every argument has been given a default value.

What that means, is that these are all the same thing.
```html
<p>{{ £my_paragraph | truncate }}</p>
<p>{{ £my_paragraph | truncate = 100 }}</p>
<p>{{ £my_paragraph | truncate = trail: ... }}</p>
<p>{{ £my_paragraph | truncate = characters: 100, trail: ... }}</p>
```
As you can see, you can pick and choose which arguments you want to overwrite - if any.

You'll have also noticed that in the second example we didn't provide a key!  
This is because, for each filter that takes arguments, one argument will be considered the "default" argument. As a result, if you provide a value, with no argument name, then this will be set to the pre-determined default argument for that filter.

### Markdowns
[Markdowns](https://daringfireball.net/projects/markdown) are simple text files that contain any text, and an optional `meta` section.

The `meta` section of the Markdown file is unique to this program.  
This section must be at the top of the document, and will be start with either `:meta` or `<meta>`, and closed with `:meta` or `</meta>`.  
It's important that if you open the meta section with `:meta`, then you must close it with `:meta`; the same is true for `<meta>` and `</meta>` otherwise the content won't be read.

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

For convenience, meta values do not need to be surrounded by quotes, they will be parsed until a new line. However, if new lines are required in a value, then the value will need to be surrounded by double quotes (`"`).  
As is standard, quotes will need to be escaped in order to prevent premature closure of the string; to do this, simply put a backslash before a double quote, like so `\"`.

```md
:meta
header = Some Company
footer = "Copyright
John \"The Mystery\" Doe"
:meta
```

#### Comments
It's possible to add comments to the meta section, by starting a line with either `#` or `//`.  
Comments will be parsed and the leading comment prefix will be removed, however this is superfluous as they will be replaced with None during parsing, and subsequently removed.
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
All generated files will be be given the exact same name as the Markdown that they are converting, but with the template's extension.  
However, if there are multiple templates, then the templates name 

By default, the file will be created in the same directory as the Markdown file, however, by providing `--output-dir` (or `-o` if that's easier) the output directory can be changed.  
This will not rename the file, but rather just place it in the specified directory.

Some formatting will be applied to the generated output, but it will likely need human intervention if you want the document to be formatted correct - such as indenting.  
Currently, a new line is placed before all headings (from `h2` to to `h6`), but nothing else is changed.

### Allow List
In some cases, this program will report warnings.

These do not prevent the program from running, but will simply print the warning to the console.  
An example of this is unused variables.

Consider if we had a Markdown file that declared the key-value pair of `author = John Doe`, and then never referenced that variable in the template, then ordinarily the template would still be created but we'd have a warning in the console like this.
```
Warning: Unused variable in 'path/to/file.md': author
```

If you do not wish for this message to be printed, you can use the following commands.
```sh
blogs-md-easy -m path/to/file.md -t path/to/template.html --allow unused
blogs-md-easy -m path/to/file.md -t path/to/template.html --allow unused_variables
```
