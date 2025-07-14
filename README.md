# Typssg

This project interfaces with the Typst Rust library to modify the HTML render to be better suited for static site generation.  Namely:

- Giving each header a proper slug id (header "Hello, world!" gets id="hello-world")
- Generating an `outline.html` which includes the document's header hierarchy as a list
- Removing the entire html header, leaving only the inside of the body without the `<body>` tag so that it can be better inserted into a webserver html template.
- Prepending some Typst redefinitions for better rendering

This project is not too generalized, as I made it for another project I'm working on.  I intend to flesh out the features of this project in the future.

