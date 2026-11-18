# kairo-java Zed extension

Contributes, without editing any upstream Zed source file:

- **Java** and **JSP** languages using pinned tree-sitter grammars.
- JSP highlighting that recognizes directives, scriptlets, EL, and tag names,
  with Java injection so JDT LS/tree-sitter Java highlighting applies inside
  scriptlets.
- **JDT LS** (`jdtls`) registration for both languages. The actual distribution
  is prepared and owned by the Kairo Go runtime agent; users can override the
  binary via `lsp.jdtls.binary` in Zed settings.
- A **`java` DAP adapter** that attaches Zed to the loopback TCP socket exposed
  by `kairo-jdi-bridge.jar` (DAP ↔ JDWP). This reuses Zed's native breakpoints,
  variables, call stack, and stepping UI.

The extension deliberately does **not** spawn `javac`, Ant, or Tomcat: those are
long-lived/stateful operations owned by `kairo_core` and the Go agent.

## Build / validate

```
cargo build -p kairo_java --target wasm32-wasip2
zed-extension --source-dir kairo/extensions/kairo-java --output-dir ... --scratch-dir ...
```

> Validation requires a Rust toolchain and the `wasm32-wasip2` target, which
> were not available in the environment where this integration was authored.
