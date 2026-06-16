#let article_card(path) = {
  let file-path = path + "index.typ"
  let content = read(file-path)

  let title = none
  for line in content.split("\n") {
    let trimmed = line.trim()
    if trimmed.starts-with("= ") {
      title = trimmed.slice(2).trim()
      break
    }
  }
  if title == none {
    panic("No level-1 heading found in " + file-path)
  }

  let img-match = content.match(regex("image\(\s*\"([^\"]+)\""))
  let image-path = if img-match != none { img-match.captures.at(0) } else { none }

  let resolved-image = if image-path == none {
    none
  } else if image-path.starts-with("/") {
    image-path
  } else {
    path + image-path
  }

  block(link(path)[
    #html.elem("div", attrs: (style:
      "display:flex; align-items:center; gap:0.8em; border:1px solid #ccc; border-radius:6px; padding:0.6em; margin:0.5rem 0;"
    ))[
      #if resolved-image != none [
        #image(resolved-image, width: 4em, height: 4em, fit: "cover")
      ]
      #strong(title)
    ]
  ])
}
