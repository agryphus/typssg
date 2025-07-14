#let image(source, width: "400px") = {
  html.elem("img", attrs: (
    src: "/static/articles/" + source,
    alt: source,
    width: str(width),
  ))
}

