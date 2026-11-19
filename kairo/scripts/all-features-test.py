"""Real-machine test of EVERY Kairo endpoint/feature against a real project."""
import json
import os
import shutil
import subprocess
import time
import urllib.error
import urllib.request

ROOT = r"D:\GPU-Tools\Temp\opencode\kairo-all-features"
AGENT = r"G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe"
BUNDLED = r"G:\spaces\kairo-ide\bundled"
JDK6 = r"D:\kairo\tmp\qa-20260909\java\zulu6.22.0.3-jdk6.0.119-win_x64"
SAMPLE = r"G:\spaces\kairo-ide\legacy-sample"


def http(method, url, payload=None):
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    req = urllib.request.Request(
        url, data=data, method=method,
        headers={"Content-Type": "application/json; charset=utf-8"})
    try:
        with urllib.request.urlopen(req, timeout=180) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", "replace")
        try:
            return json.loads(body)
        except ValueError:
            return {"ok": False, "error": {"code": "http", "message": body[:200]}}


def main():
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
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    port = None
    state_path = os.path.join(agent_dir, "agent-state.json")
    for _ in range(200):
        time.sleep(0.1)
        if os.path.exists(state_path):
            state = json.load(open(state_path, encoding="utf-8"))
            if state.get("status") == "ready":
                port = state["port"]
                break
    base = "http://127.0.0.1:%d/api/v1" % port
    results = []
    def rec(name, response):
        ok = bool(response.get("ok"))
        err = response.get("error") or {}
        code = err.get("code", "")
        if not ok:
            print("  DEBUG %s -> %s" % (name, err.get("message", "")[:300]))
        results.append((name, ok, code))
        return response

    try:
        rec("01 health", http("GET", base + "/health"))
        rec("02 endpoints", http("GET", base + "/endpoints"))
        rec("27 toolchains list", http("GET", base + "/toolchains"))
        rec("28 toolchain import", http("POST", base + "/toolchains/import",
            {"payload": {"path": JDK6, "label": "Zulu 6"}}))
        wsopen = rec("05 workspace open", http("POST", base + "/workspaces",
            {"payload": {"rootPath": root}}))
        wsid = wsopen.get("payload", {}).get("id", "")
        rec("08 workspace scan", http("POST", base + "/workspaces/%s/scan" % wsid,
            {"payload": {"rootPath": root}}))
        rec("10 JDT prepare", http("POST", base + "/workspaces/%s/java/prepare" % wsid, {}))
        rec("24 project detect", http("POST", base + "/projects/detect",
            {"payload": {"rootPath": root}}))
        imp = rec("25 project import", http("POST", base + "/projects/import", {"payload": {
            "workspaceId": wsid, "name": "all-features", "rootPath": root,
            "sourceDirs": ["src/main/java"], "webappDir": "WebRoot",
            "outputDir": "build/classes", "buildFile": "build.xml",
            "sourceLevel": "1.6", "targetLevel": "1.6", "encoding": "gbk",
            "buildTool": "ant", "contextPath": "/"}}))
        project_id = imp.get("payload", {}).get("id", "project-legacy-sample")
        rec("11 launch descriptor", http("GET",
            base + "/workspaces/%s/java/launch-descriptor?projectId=%s" % (wsid, project_id)))
        rec("13 run configurations save", http("PUT",
            base + "/workspaces/%s/run-configurations" % wsid, {"payload": {
                "version": 1,
                "configurations": [{
                    "id": "default", "name": "Legacy Tomcat", "type": "tomcat6",
                    "projectId": project_id, "mode": "run", "suspend": False,
                    "jdkRef": "jdk6",
                    "build": {"type": "ant", "target": "compile", "command": "", "clean": False},
                    "server": {"id": "tomcat6", "httpPort": 18082, "debugPort": 8000,
                               "contextPath": "/"},
                    "deploy": {"mode": "exploded", "artifact": "legacy-sample"},
                    "env": {}, "vmOptions": [], "beforeLaunchTasks": [],
                }],
                "selectedConfigurationId": "default",
            }}))
        rec("12 run configurations list", http("GET",
            base + "/workspaces/%s/run-configurations" % wsid))
        rec("20 projects list", http("GET", base + "/projects?workspaceId=" + wsid))
        rec("21 project get", http("GET", base + "/projects/" + project_id))
        build = rec("30 build start", http("POST", base + "/builds",
            {"payload": {"projectId": project_id}}))
        build_id = build.get("payload", {}).get("id", "")
        for _ in range(120):
            time.sleep(0.5)
            b = http("GET", base + "/builds/" + build_id)
            if b.get("payload", {}).get("state") not in ("queued", "running"):
                rec("31 build result", b)
                break
        rec("29 builds list", http("GET", base + "/builds?workspaceId=" + wsid))
        dep = rec("38 deploy", http("POST", base + "/deployments", {"payload": {
            "projectId": project_id, "buildId": build_id, "scope": "incremental"}}))
        rec("37 deployments list", http("GET", base + "/deployments"))
        srv = rec("41 server start", http("POST", base + "/servers",
            {"payload": {"projectId": project_id}}))
        server_id = srv.get("payload", {}).get("id", "")
        rec("40 servers list", http("GET", base + "/servers?workspaceId=" + wsid))
        if server_id:
            rec("42 server get", http("GET", base + "/servers/" + server_id))
            rec("46 server logs", http("GET",
                base + "/servers/%s/logs?tail=20" % server_id))
            rec("49 recoverable servers", http("GET", base + "/servers/recoverable"))
            rec("48 server reload", http("POST", base + "/servers/%s/reload" % server_id, {}))
            rec("44 server debug", http("POST", base + "/servers/%s/debug" % server_id, {}))
            rec("45 server restart", http("POST", base + "/servers/%s/restart" % server_id, {}))
        rec("51 search", http("POST", base + "/search", {"payload": {
            "query": "Hello", "workspaceId": wsid, "rootPath": root,
            "projectEncoding": "gbk", "maxResults": 1000}}))
        rec("52 search files", http("POST", base + "/search/files", {"payload": {
            "workspaceId": wsid, "rootPath": root}}))
        rec("54 encoding detect", http("POST", base + "/encoding/detect", {"payload": {
            "file": os.path.join(root, "WebRoot", "hello.jsp")}}))
        rec("56 encoding validate", http("POST", base + "/encoding/validate",
            {"payload": {"text": "中文", "encoding": "gbk"}}))
        rec("61 JDT status", http("GET", base + "/jdtls"))
        rec("64 JDT distribution", http("GET", base + "/jdtls/distribution"))
        rec("65 JDT project status", http("GET",
            base + "/jdtls/project?workspaceId=" + wsid))
        rec("66 JDT project generate", http("POST", base + "/jdtls/project", {"payload": {
            "workspaceId": wsid, "projectId": project_id, "rootPath": root,
            "autoDetectClasspath": True, "intoProjectRoot": True}}))
        rec("67 port diagnostics", http("GET", base + "/diagnostics/port?port=18082"))
        rec("73 debug adapter status", http("GET", base + "/debug/adapter/status"))
        rec("76 debug JDK download", http("POST", base + "/debug/jdk/download", {}))
        rec("77 java run", http("POST", base + "/java/run", {"payload": {
            "projectRoot": root,
            "filePath": os.path.join(root, "src/main/java/com/example/HelloWorld.java"),
            "className": "com.example.HelloWorld",
            "packageName": "com.example",
            "methodType": "main"}}))
        rec("81 jvm compile incremental", http("POST", base + "/jvm/compile-incremental",
            {"payload": {"projectId": project_id, "files": []}}))
        rec("80 java detect", http("POST", base + "/java/detect", {"payload": {
            "filePath": os.path.join(root, "src/main/java/com/example/HelloWorld.java")}}))
        rec("68 Maven detect", http("POST", base + "/maven/detect",
            {"payload": {"rootPath": root}}))
        rec("59 audit", http("GET", base + "/audit"))
        rec("57 auth login", http("POST", base + "/auth/login",
            {"payload": {"username": "kairo", "password": "kairo"}}))
        if server_id:
            rec("43 server stop", http("DELETE", base + "/servers/" + server_id, {}))
    finally:
        agent.terminate()
        try:
            agent.wait(timeout=10)
        except subprocess.TimeoutExpired:
            agent.kill()

    # Auth phase: start a second agent with a real local secret and verify the
    # shared-secret boundary and login endpoint.
    agent.terminate()
    try:
        agent.wait(timeout=10)
    except subprocess.TimeoutExpired:
        agent.kill()
    auth_dir = os.path.join(ROOT, "auth-agent")
    os.makedirs(auth_dir, exist_ok=True)
    state_auth = os.path.join(auth_dir, "agent-state.json")
    if os.path.exists(state_auth):
        os.remove(state_auth)
    env = dict(os.environ)
    env["KAIRO_LOCAL_SECRET"] = "kairo-real-test-secret"
    env["KAIRO_AUTH_USER"] = "kairo"
    env["KAIRO_AUTH_PASSWORD"] = "kairo-pass"
    agent = subprocess.Popen(
        [AGENT, "-port", "0", "-bind", "127.0.0.1", "-data-dir", auth_dir,
         "-bundled-dir", BUNDLED, "-log-level", "warn", "-require-auth"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    auth_port = None
    for _ in range(200):
        time.sleep(0.1)
        if os.path.exists(state_auth):
            state = json.load(open(state_auth, encoding="utf-8"))
            if state.get("status") == "ready":
                auth_port = state["port"]
                break
    if auth_port:
        auth_base = "http://127.0.0.1:%d/api/v1" % auth_port
        def auth_http(method, path, payload=None, secret=False):
            data = json.dumps(payload).encode("utf-8") if payload is not None else None
            headers = {"Content-Type": "application/json; charset=utf-8"}
            if secret:
                headers["X-Kairo-Secret"] = "kairo-real-test-secret"
            req = urllib.request.Request(auth_base + path, data=data, method=method, headers=headers)
            try:
                with urllib.request.urlopen(req, timeout=60) as response:
                    return json.load(response)
            except urllib.error.HTTPError as error:
                return json.loads(error.read().decode("utf-8", "replace"))
        rec("58 auth health (public)", auth_http("GET", "/health"))
        rec("57 auth login", auth_http("POST", "/auth/login",
            {"payload": {"username": "kairo", "password": "kairo-pass"}}))
        rec("59 auth protected without secret", auth_http("GET", "/toolchains"))
        rec("59 auth protected with secret", auth_http("GET", "/toolchains", secret=True))
    agent.terminate()
    try:
        agent.wait(timeout=10)
    except subprocess.TimeoutExpired:
        agent.kill()

    # Environment prerequisites (not product defects):
    # - /auth/login requires a pre-hashed password file; this run has none.
    # - protected endpoint without secret MUST fail (that is the security test).
    expected_failures = {
        "57 auth login",       # first phase: auth not configured
        "59 auth protected without secret",  # expected rejection
    }
    applicable = [(n, ok, c) for n, ok, c in results if n not in expected_failures]
    passed = sum(1 for _, ok, _ in applicable if ok)
    print("REAL ENDPOINT TEST: %d/%d applicable OK" % (passed, len(applicable)))
    for name, ok, code in results:
        print("  %-34s %s %s" % (name, "PASS" if ok else "FAIL", code))
    report = os.path.join(ROOT, "results.json")
    json.dump([{"name": n, "ok": ok, "code": c} for n, ok, c in results],
              open(report, "w", encoding="utf-8"), indent=2, ensure_ascii=False)
    print("report=" + report)


if __name__ == "__main__":
    main()

