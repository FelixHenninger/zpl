#let mk_label(content) = [
  #let printer = json(bytes(sys.inputs.at("label")))

  #set page(width: printer.at("width")*1mm, height: printer.at("height")*1mm)

  #content
]
