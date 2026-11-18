; Java scriptlets/directives inside a JSP are parsed as Java so JDT LS and
; tree-sitter-java highlighting apply to `<% ... %>` bodies.
((jsp) @injection.content
  (#set! injection.language "java"))

; JSP EL expressions (${...}, <%= ... %>) are highlighted as embedded code.
((interpolation) @injection.content
  (#set! injection.language "java"))
