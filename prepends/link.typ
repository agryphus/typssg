// Adds an "internal" class to links that stay within the application,
// allowing them to be styled differently from external links.
#show link: it => {
  let dest = it.dest
  if type(dest) == str {
    // Check if body is text-only (no images or other non-text elements)
    let is-text-only(content) = {
      let sequence = [a b].func()
      let allowed = (text, strong, emph)
      if content.func() == text { return true }
      if content.func() in allowed {
        if content.has("body") {
          return is-text-only(content.body)  // recurse into strong/emph
        }
        return true
      }
      if content.func() == sequence {
        return content.children.all(c => is-text-only(c))
      }
      false
    }

    let is-internal = dest.starts-with("/") or dest.starts-with(".")
    let attrs = (href: dest)
    if is-internal and is-text-only(it.body) {
      attrs = (href: dest, class: "internal")
    }
    html.elem("a", attrs: attrs, it.body)
  } else {
    it
  }
}
