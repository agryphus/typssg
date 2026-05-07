// Adds an "internal" class to links that stay within the application,
// allowing them to be styled differently from external links.
#show link: it => {
  let dest = it.dest
  if type(dest) == str {
    let is-internal = dest.starts-with("/") or dest.starts-with(".")
    let attrs = (href: dest)
    if is-internal {
      attrs = (href: dest, class: "internal")
    }
    html.elem("a", attrs: attrs, it.body)
  } else {
    it
  }
}
