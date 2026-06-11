#let article_link(path) = {
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

  link(path)[#title]
}
