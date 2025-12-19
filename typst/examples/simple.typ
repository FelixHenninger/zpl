#let mk_label() = [
  #let printer = json(bytes(sys.inputs.at("label")))

  #set page(width: printer.at("width")*1mm, height: printer.at("height")*1mm)
  // #set page(width: 50mm, height: 50mm, margin: 0pt)
  #set align(center)
  #v(30%)

  // #str((printer.at("width")*1mm).mm())
  // #str((printer.at("height")*1mm).mm())
  Hello, world!
]

#mk_label()
