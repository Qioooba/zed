(line_comment) @comment
(block_comment) @comment
(string_literal) @string
(character_literal) @string
(true) @constant
(false) @constant
(null_literal) @constant
(decimal_integer_literal) @number
(hex_integer_literal) @number
(decimal_floating_point_literal) @number
(hex_floating_point_literal) @number
(identifier) @variable
(type_identifier) @type
(class_declaration name: (identifier) @type)
(interface_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(method_declaration name: (identifier) @function)
[
  "public"
  "private"
  "protected"
  "static"
  "final"
  "abstract"
  "synchronized"
  "native"
  "transient"
  "volatile"
  "class"
  "interface"
  "enum"
  "extends"
  "implements"
  "return"
  "if"
  "else"
  "for"
  "while"
  "do"
  "switch"
  "case"
  "break"
  "continue"
  "new"
  "throw"
  "throws"
  "try"
  "catch"
  "finally"
  "import"
  "package"
] @keyword
