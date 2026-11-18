"""Validate .scm query node names against a pinned tree-sitter grammar's node-types.json."""
import base64
import json
import re
import sys
import urllib.request

GRAMMARS = {
    "jsp": (
        "merico-ai/tree-sitter-jsp",
        "e02ba40fefd64776b56e0d19d7771e6aadab3ec4",
        r"E:\github\zed\kairo\extensions\kairo-java\languages\jsp",
    ),
    "java": (
        "tree-sitter/tree-sitter-java",
        "e10607b45ff745f5f876bfa3e94fbcc6b44bdc11",
        r"E:\github\zed\kairo\extensions\kairo-java\languages\java",
    ),
}


def fetch_node_types(repo, rev):
    url = (
        "https://api.github.com/repos/%s/contents/src/node-types.json?ref=%s"
        % (repo, rev)
    )
    request = urllib.request.Request(url, headers={"User-Agent": "kairo-query-validator"})
    with urllib.request.urlopen(request, timeout=60) as response:
        payload = json.load(response)
    raw = base64.b64decode(payload["content"])
    return {item["type"] for item in json.loads(raw)}


def referenced_names(path):
    text = open(path, encoding="utf-8").read()
    # named nodes: (name ...) - ignore query predicates and comments
    named = set(re.findall(r"\(\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:\{|\)|\()", text))
    # anonymous token literals: ("..." ...)
    literals = set(re.findall(r'"([^"\n]+)"', text))
    return named, literals


def main():
    failures = 0
    for language, (repo, rev, directory) in GRAMMARS.items():
        try:
            types = fetch_node_types(repo, rev)
        except Exception as error:  # noqa: BLE001
            print("%s: could not fetch node-types (%s)" % (language, error))
            failures += 1
            continue
        import os
        for file_name in ("highlights.scm", "injections.scm"):
            path = os.path.join(directory, file_name)
            if not os.path.exists(path):
                continue
            named, literals = referenced_names(path)
            missing_named = sorted(name for name in named if name not in types)
            missing_literals = sorted(
                value for value in literals if value not in types and len(value) > 1
            )
            print(
                "%s/%s: %d named nodes, %d literals checked"
                % (language, file_name, len(named), len(literals))
            )
            if missing_named:
                print("  MISSING NAMED: %s" % ", ".join(missing_named))
                failures += 1
            if missing_literals:
                print("  LITERALS NOT IN GRAMMAR (may still be valid tokens): %s"
                      % ", ".join(missing_literals))
    print("RESULT: %s" % ("PASS" if failures == 0 else "FAIL (%d)" % failures))


if __name__ == "__main__":
    main()
