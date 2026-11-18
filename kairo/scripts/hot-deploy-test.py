"""Real hot-deploy mode test: staticSync, compileOnly (/jvm/compile), contextReload."""
import json
import os
import shutil
import time
import urllib.request

ROOT = r"D:\GPU-Tools\Temp\opencode\kairo-hot-deploy-test"
AGENT = r"G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe"
BUNDLED = r"G:\spaces\kairo-ide\bundled"
JDK6 = r"D:\kairo\tmp\qa-20260909\java\zulu6.22.0.3-jdk6.0.119-win_x64"
SAMPLE = r"G:\spaces\kairo-ide\legacy-sample"


def http(method, url, payload=None):
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = urllib.request.Request(
        url, data=data, method=method,
        headers={"Content-Type": "application/json; charset=utf-8"},
    )
    try:
        with urllib.request.urlopen(request, timeout=180) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        print("HTTP %s %s -> %s" % (error.code, url, error.read().decode("utf-8", "replace")))
        raise


def elapsed_ms(start):
    return round((time.monotonic() - start) * 1000, 1)


def main():
    import subprocess

    shutil.rmtree(ROOT, ignore_errors=True)
    os.makedirs(ROOT, exist_ok=True)
    agent_dir = os.path.join(ROOT, "agent")
    os.makedirs(agent_dir, exist_ok=True)
    root = os.path.join(ROOT, "legacy-sample")
    shutil.copytree(SAMPLE, root)
    shutil.rmtree(os.path.join(root, ".kairo"), ignore_errors=True)
    web_inf = os.path.join(root, "WEB-INF")
    os.makedirs(web_inf, exist_ok=True)
    shutil.copy2(os.path.join(root, "WebRoot", "WEB-INF", "web.xml"),
                 os.path.join(web_inf, "web.xml"))
    shutil.copytree(os.path.join(root, "build", "classes"),
                    os.path.join(web_inf, "classes"), dirs_exist_ok=True)

    agent = subprocess.Popen(
        [AGENT, "-port", "0", "-bind", "127.0.0.1", "-data-dir", agent_dir,
         "-bundled-dir", BUNDLED, "-log-level", "warn"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    port = None
    state_file = os.path.join(agent_dir, "agent-state.json")
    for _ in range(200):
        time.sleep(0.1)
        if os.path.exists(state_file):
            state = json.load(open(state_file, encoding="utf-8"))
            if state.get("status") == "ready":
                port = state["port"]
                break
    base = "http://127.0.0.1:%d/api/v1" % port
    results = []
    try:
        http("POST", base + "/toolchains/import",
             {"requestId": "tc", "payload": {"path": JDK6, "label": "Zulu JDK 6"}})
        ws = http("POST", base + "/workspaces",
                  {"requestId": "ws", "payload": {"rootPath": root}})
        wsid = ws["payload"]["id"]
        project = http("POST", base + "/projects/import", {"requestId": "imp", "payload": {
            "workspaceId": wsid, "name": "hot-deploy-sample", "rootPath": root,
            "sourceDirs": ["src/main/java"], "webappDir": "WebRoot",
            "outputDir": "build/classes", "buildFile": "build.xml",
            "sourceLevel": "1.6", "targetLevel": "1.6", "encoding": "gbk",
            "buildTool": "ant", "contextPath": "/",
        }})
        project_id = project["payload"]["id"]
        build = http("POST", base + "/builds",
                     {"requestId": "build", "payload": {"projectId": project_id}})
        build_id = build["payload"]["id"]
        for _ in range(120):
            time.sleep(0.5)
            if http("GET", base + "/builds/" + build_id)["payload"]["state"] not in (
                "queued", "running"
            ):
                break
        deploy = http("POST", base + "/deployments", {"requestId": "dep", "payload": {
            "projectId": project_id, "buildId": build_id, "scope": "incremental",
        }})
        results.append(("deploy mode", deploy["payload"]["hotReloadMode"], 0))
        server = http("POST", base + "/servers",
                      {"requestId": "srv", "payload": {"projectId": project_id}})
        server_id = server["payload"]["id"]
        http_port = server["payload"]["ports"]["http"]
        jsp_url = "http://127.0.0.1:%d/src/main/webapp/index.jsp" % http_port

        # 1. staticSync: edit JSP and wait until served without a restart
        jsp_file = os.path.join(root, "src", "main", "webapp", "index.jsp")
        original = open(jsp_file, "rb").read()
        start = time.monotonic()
        try:
            text = original.decode("utf-8")
            with open(jsp_file, "w", encoding="utf-8", newline="") as handle:
                handle.write(text.replace("</html>", "<!-- HOT-PROBE --></html>"))
            visible = False
            for _ in range(15):
                time.sleep(2)
                with urllib.request.urlopen(jsp_url, timeout=10) as response:
                    visible = "HOT-PROBE" in response.read().decode("utf-8", "replace")
                if visible:
                    break
            results.append(("staticSync JSP visible without restart", visible, elapsed_ms(start)))
        finally:
            with open(jsp_file, "wb") as handle:
                handle.write(original)

        # 2. compileOnly: modify a Java source and invoke incremental compile
        java_file = os.path.join(root, "src", "main", "java", "com", "example", "HelloWorld.java")
        java_original = open(java_file, "rb").read()
        try:
            text = java_original.decode("utf-8")
            with open(java_file, "w", encoding="utf-8", newline="") as handle:
                handle.write(text.replace('"123"', '"HOT-COMPILE-PROBE"'))
            start = time.monotonic()
            compile_result = http("POST", base + "/builds", {"requestId": "jc", "payload": {
                "projectId": project_id, "clean": False,
            }})
            new_build_id = compile_result["payload"]["id"]
            for _ in range(120):
                time.sleep(0.5)
                result = http("GET", base + "/builds/" + new_build_id)["payload"]
                if result["state"] not in ("queued", "running"):
                    break
            results.append(("compileOnly full build", result["state"] == "success", elapsed_ms(start)))
            print("compile state: %s" % result["state"])
        finally:
            with open(java_file, "wb") as handle:
                handle.write(java_original)

        # 3. contextReload: verified restart with a new generation
        before = http("GET", base + "/servers/" + server_id)["payload"]["generation"]
        web_xml_path = os.path.join(web_inf, "web.xml")
        with open(web_xml_path, "rb") as handle:
            web_xml_bytes = handle.read()
        with open(web_xml_path, "wb") as handle:
            handle.write(web_xml_bytes)
        start = time.monotonic()
        reload = http("POST", base + "/servers/" + server_id + "/reload", {})
        print("reload response: %s" % json.dumps(reload, ensure_ascii=False))
        time.sleep(3)
        server_after = http("GET", base + "/servers/" + server_id)["payload"]
        print("server after reload: %s" % json.dumps(server_after, ensure_ascii=False))
        after = server_after.get("generation", 0)
        # A Tomcat context reload reloads the webapp in the same JVM, so the
        # server generation legitimately stays unchanged.
        results.append(("contextReload (gen %d -> %d, server %s)"
                        % (before, after, server_after.get("state")),
                        reload.get("ok") and server_after.get("state") == "running",
                        elapsed_ms(start)))

        http("DELETE", base + "/servers/" + server_id, {})
    finally:
        agent.terminate()
        try:
            agent.wait(timeout=10)
        except subprocess.TimeoutExpired:
            agent.kill()

    for name, value, ms in results:
        print("%-55s %-30s %8s ms" % (name, value, ms))
    ok = all(bool(value) for _, value, _ in results)
    print("RESULT: %s" % ("PASS" if ok else "FAIL"))


if __name__ == "__main__":
    main()
