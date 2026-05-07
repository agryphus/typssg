// By default, bibliography titles render with a level 1 heading.  Typssg
// rather assumes that lvl 1 headings are just for page titles, and that
// section titles are denoted with lvl 2 headings.
#let _bibliography = bibliography
#let refs(content) = {
  heading(level: 2, "References")
  _bibliography(bytes(content.text), style: "ieee", title: none)
}

// Override default bibliography so that users don't accidentally use it
#let bibliography = (path, ..args) => {
  text[This project has enabled the bibliography extension which provides the `#refs()` funciton for convenient in-file bibliographies.  If you _did_ intend to use the default bibliography function, call it instead with `#_bibliography().`]
}

// Have in-text citations appear as just numbers, instead of the IEEE default
// style of [#] with brackets.
#set cite(style: "vancouver-superscript")
