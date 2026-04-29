// Given a figure with an image, this show rule will wrap the image in a link
// that goes to the full version of the pic.  For example, image("bird.jpg")
// will get wrapped in a #link("bird_full.jpg").  This, of course, assumes
// bird_full.jpg also exists in the directory.
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
