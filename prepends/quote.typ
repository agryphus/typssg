// Typst version 14.0.2 outputs a block quote as a <blockquote> followed by
// a <p> for the attribution.  This makes it difficult to target the
// attribution for styling.  This snippet instead uses a <blockquote> and
// a <figcaption> wrapped in a <figure> block.
#show quote.where(block: true): it => {
  let inner = html.elem("blockquote", it.body)
  if it.attribution != none {
    html.elem("figure", {
      inner
      html.elem("figcaption", it.attribution)
    })
  } else {
    inner
  }
}
