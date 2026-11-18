; Comments and documentation
(line_comment) @comment
(block_comment) @comment

; Literals
(string_literal) @string
(character_literal) @string
(true) @constant
(false) @constant
(null_literal) @constant
(this) @constant
(super) @constant
(decimal_integer_literal) @number
(hex_integer_literal) @number
(decimal_floating_point_literal) @number
(hex_floating_point_literal) @number

; Type and symbol names
(type_identifier) @type
(scoped_type_identifier) @type
(class_declaration name: (identifier) @type)
(interface_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(annotation_type_declaration name: (identifier) @type)
(method_declaration name: (identifier) @function)
(constructor_declaration name: (identifier) @function)
(method_invocation name: (identifier) @function)
(field_declaration declarator: (variable_declarator name: (identifier) @variable))
(local_variable_declaration declarator: (variable_declarator name: (identifier) @variable))
(formal_parameter name: (identifier) @variable)
(identifier) @variable

; Annotations and imports
(annotation name: (identifier) @attribute)
(marker_annotation name: (identifier) @attribute)
(import_declaration) @module
(package_declaration) @module

; Keywords — anonymous tokens of tree-sitter-java
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
  "strictfp"
  "default"
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
  "instanceof"
  "assert"
] @keyword

(boolean_type) @type
(void_type) @type
(integral_type) @type
(floating_point_type) @type
