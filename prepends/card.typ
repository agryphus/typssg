#let card(
  caption: "",
  media: (),
  score: 0,
  defense: "",
  offense: "",
) = {
  text[= #caption (Card)

  #html.elem("div", attrs: (class: "card"))[
    #media.at(0)
  ]

  score: #score

  == Defense
  #defense

  == Offense
  #offense
  ]
}
