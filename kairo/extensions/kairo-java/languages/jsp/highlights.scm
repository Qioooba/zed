; JSP grammar nodes verified against merico-ai/tree-sitter-jsp node-types.json
(comment) @comment
(doc_type) @constant
(directive_name) @keyword
(directive_modifier) @keyword
(directive_modifiers) @keyword
(directive_argument) @variable
(directive_attribute) @attribute
(directive_dynamic_argument) @variable
(directive_dynamic_argument_value) @string
(attribute_name) @attribute
(attribute_value) @string
(quoted_attribute_value) @string
(tag_name) @tag
(start_tag) @tag
(end_tag) @tag
(self_closing_tag) @tag
(element) @tag
(component) @tag
(script_element) @tag
(style_element) @tag
(template_element) @tag
(text) @text
(jsp) @embedded
(interpolation) @embedded
(raw_text) @embedded
(erroneous_end_tag_name) @invalid

; Punctuation / DSL delimiters
"{{" @punctuation
"}}" @punctuation
"</" @punctuation
"/>" @punctuation
