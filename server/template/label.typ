#let mk_label(content) = [
  #let printer = json(bytes(sys.inputs.at("label")))
  #set page(
    width: printer.at("width")*1mm,
    height: printer.at("height")*1mm,
    margin: (
      left: printer.at("margin_left")*1mm,
      right: printer.at("margin_right")*1mm,
      top: printer.at("margin_top")*1mm,
      bottom: printer.at("margin_bottom")*1mm
    )
  )

  #content
]

#let mk_logo() = {
  let logo_bytes = read("logo.svg+xml", encoding: none)
  image(logo_bytes, format: "svg", fit: "contain")
}
