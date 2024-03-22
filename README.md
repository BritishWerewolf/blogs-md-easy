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
Usage: blogs-md-easy.exe --template <FILE> --markdowns <FILES>...

Options:
  -t, --template <FILE>       HTML template that the Markdowns will populate
  -m, --markdowns <FILES>...  List of Markdown files ending in .md
  -h, --help                  Print help
  -V, --version               Print version
```

## Template
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

## Markdowns
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

## Output
All generated HTML files will be placed in the same directory as the markdown, using the same file name but with the `.html` extension.

Some formatting will be applied to the generated output, but it will likely need human intervention if you want the document to be formatted correct - such as indenting.

## Todo List
- [ ] Generate for multiple templates at once.
- [ ] Option to run against markdowns to determine if they were built off a different template.
- [ ] Add comments to the meta section.
- [ ] Formatting of the generated file.
- [ ] Document all functions.
- [ ] Reduce complexity of `main` function, to make it easier to test.
