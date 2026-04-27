#show figure: it => {
  if it.body.func() == image {
    let src = it.body.source
    let dot-pos = src.rev().position(".")
    let full-src = if dot-pos != none {
      src.slice(0, src.len() - dot-pos - 1) + "_full" + src.slice(src.len() - dot-pos - 1)
    } else {
      src + "_full"
    }
    show image: img => link(full-src, img)
    it
  } else {
    it
  }
}
