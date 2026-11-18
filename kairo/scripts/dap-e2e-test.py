"""Real DAP/JDWP end-to-end test for the Kairo JDI bridge.

Drives kairo-runtime.exe over /api/v1, starts Tomcat 6 with JDWP, spawns
kairo-jdi-bridge.jar with DAP-over-stdio, sets a breakpoint in HelloServlet,
triggers the servlet over HTTP, and verifies stop -> stack -> locals -> continue.
"""
import json
import os
import queue
import shutil
import subprocess
import sys
import threading
import time
import urllib.request

ROOT = r"D:\GPU-Tools\Temp\opencode\kairo-dap-test"
AGENT = r"G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe"
BUNDLED = r"G:\spaces\kairo-ide\bundled"
BRIDGE = r"G:\spaces\kairo-ide\bundled\kairo-jdi-bridge.jar"
JDK6 = r"D:\kairo\tmp\qa-20260909\java\zulu6.22.0.3-jdk6.0.119-win_x64"
SAMPLE = r"G:\spaces\kairo-ide\legacy-sample"


def http(method, url, payload=None):
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={"Content-Type": "application/json; charset=utf-8"},
    )
    with urllib.request.urlopen(request, timeout=180) as response:
        return json.load(response)


class DapClient:
    def __init__(self, proc):
        self.proc = proc
        self.seq = 0
        self.messages = queue.Queue()
        threading.Thread(target=self._reader, daemon=True).start()

    def _reader(self):
        while True:
            header = b""
            while b"\r\n\r\n" not in header:
                byte = self.proc.stdout.read(1)
                if not byte:
                    return
                header += byte
            length = 0
            for line in header.decode("ascii").split("\r\n"):
                if line.lower().startswith("content-length:"):
                    length = int(line.split(":", 1)[1].strip())
            body = self.proc.stdout.read(length)
            if not body:
                return
            self.messages.put(json.loads(body.decode("utf-8")))

    def send(self, command, arguments=None):
        self.seq += 1
        message = {
            "seq": self.seq,
            "type": "request",
            "command": command,
            "arguments": arguments or {},
        }
        body = json.dumps(message).encode("utf-8")
        self.proc.stdin.write(
            ("Content-Length: %d\r\n\r\n" % len(body)).encode("ascii") + body
        )
        self.proc.stdin.flush()
        return self.seq

    def wait_response(self, request_seq, timeout=60):
        deadline = time.time() + timeout
        while time.time() < deadline:
            message = self.messages.get(timeout=max(0.1, deadline - time.time()))
            if (
                message.get("type") == "response"
                and message.get("request_seq") == request_seq
            ):
                return message
        raise TimeoutError("no response for %s" % request_seq)

    def wait_event(self, event_type, timeout=60):
        deadline = time.time() + timeout
        while time.time() < deadline:
            message = self.messages.get(timeout=max(0.1, deadline - time.time()))
            if message.get("type") == "event" and message.get("event") == event_type:
                return message
        raise TimeoutError("no event %s" % event_type)


def main():
    shutil.rmtree(ROOT, ignore_errors=True)
    os.makedirs(ROOT, exist_ok=True)
    agent_dir = os.path.join(ROOT, "agent")
    os.makedirs(agent_dir, exist_ok=True)
    project_root = os.path.join(ROOT, "legacy-sample")
    shutil.copytree(SAMPLE, project_root)
    shutil.rmtree(os.path.join(project_root, ".kairo"), ignore_errors=True)

    agent = subprocess.Popen(
        [AGENT, "-port", "0", "-bind", "127.0.0.1", "-data-dir", agent_dir,
         "-bundled-dir", BUNDLED, "-log-level", "warn"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    port = None
    state = os.path.join(agent_dir, "agent-state.json")
    for _ in range(200):
        time.sleep(0.1)
        if os.path.exists(state):
            data = json.load(open(state, encoding="utf-8"))
            if data.get("status") == "ready":
                port = data["port"]
                break
    if not port:
        raise RuntimeError("agent did not start")
    base = "http://127.0.0.1:%d/api/v1" % port
    results = {}

    tc = http("POST", base + "/toolchains/import",
              {"requestId": "tc", "payload": {"path": JDK6, "label": "Zulu JDK 6"}})
    results["toolchain"] = tc["ok"]
    ws = http("POST", base + "/workspaces",
              {"requestId": "ws", "payload": {"rootPath": project_root}})
    wsid = ws["payload"]["id"]
    project = http("POST", base + "/projects/import", {"requestId": "imp", "payload": {
        "workspaceId": wsid, "name": "dap-sample", "rootPath": project_root,
        "sourceDirs": ["src/main/java"], "webappDir": "WebRoot",
        "outputDir": "build/classes", "buildFile": "build.xml",
        "sourceLevel": "1.6", "targetLevel": "1.6", "encoding": "gbk",
        "buildTool": "ant", "contextPath": "/",
    }})
    project_id = project["payload"]["id"]
    build = http("POST", base + "/builds",
                 {"requestId": "build", "payload": {"projectId": project_id}})
    build_id = build["payload"]["id"]
    state_value = ""
    for _ in range(120):
        time.sleep(0.5)
        state_value = http("GET", base + "/builds/" + build_id)["payload"]["state"]
        if state_value not in ("queued", "running"):
            break
    results["build"] = state_value
    # The agent mounts the project root as docBase; mirror the legacy web.xml
    # so the /hello servlet mapping is visible.
    web_inf = os.path.join(project_root, "WEB-INF")
    os.makedirs(web_inf, exist_ok=True)
    shutil.copy2(
        os.path.join(project_root, "WebRoot", "WEB-INF", "web.xml"),
        os.path.join(web_inf, "web.xml"),
    )
    shutil.copytree(
        os.path.join(project_root, "build", "classes"),
        os.path.join(web_inf, "classes"),
        dirs_exist_ok=True,
    )
    server = http("POST", base + "/servers",
                  {"requestId": "srv", "payload": {"projectId": project_id, "debug": True}})
    server_id = server["payload"]["id"]
    jdwp_port = server["payload"].get("ports", {}).get("debug", 0)
    http_port = server["payload"]["ports"]["http"]
    results["server"] = server["payload"]["state"]
    results["jdwp_port"] = jdwp_port
    print("jdwp=%s http=%s" % (jdwp_port, http_port))

    trace_path = os.path.join(ROOT, "jdi-trace.log")
    bridge_env = dict(os.environ)
    bridge_env["KAIRO_JDI_TRACE"] = trace_path
    bridge = subprocess.Popen(
        ["java", "-jar", BRIDGE, "127.0.0.1", str(jdwp_port)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        env=bridge_env,
    )
    try:
        client = DapClient(bridge)
        seq = client.send("initialize", {"clientID": "kairo-zed-test", "adapterID": "java"})
        response = client.wait_response(seq)
        results["initialize"] = response.get("success")
        client.wait_event("initialized", timeout=20)

        seq = client.send("attach", {
            "hostName": "127.0.0.1",
            "port": jdwp_port,
            "sourcePaths": [os.path.join(project_root, "src", "main", "java")],
        })
        results["attach"] = client.wait_response(seq).get("success")

        source_path = os.path.join(
            project_root, "src", "main", "java", "com", "example", "legacy", "HelloServlet.java"
        )
        seq = client.send("setBreakpoints", {
            "source": {"path": source_path, "name": "HelloServlet.java"},
            "breakpoints": [{"line": 26}],
        })
        bp_response = client.wait_response(seq)
        results["breakpoints"] = bp_response.get("body", {}).get("breakpoints", [])
        seq = client.send("configurationDone")
        results["configurationDone"] = client.wait_response(seq).get("success")

        # Trigger the servlet that contains the breakpoint.
        try:
            with urllib.request.urlopen(
                "http://127.0.0.1:%d/hello?name=Kairo" % http_port, timeout=30
            ) as response:
                body = response.read().decode("utf-8", "replace")
                results["http_response"] = "Kairo" in body
        except Exception as error:  # noqa: BLE001
            results["http_response"] = str(error)

        print("pre-stop results: %s" % json.dumps(results, ensure_ascii=False))
        stopped = client.wait_event("stopped", timeout=60)
        thread_id = stopped["body"]["threadId"]
        results["stopped"] = True
        seq = client.send("stackTrace", {"threadId": thread_id})
        frames = client.wait_response(seq)["body"]["stackFrames"]
        results["top_frame"] = frames[0].get("name") if frames else None
        frame_id = frames[0]["id"] if frames else -1
        seq = client.send("scopes", {"frameId": frame_id})
        scopes = client.wait_response(seq)["body"]["scopes"]
        variables_ref = scopes[0]["variablesReference"] if scopes else -1
        seq = client.send("variables", {"variablesReference": variables_ref})
        variables = client.wait_response(seq)["body"]["variables"]
        name_var = next((v for v in variables if v.get("name") == "name"), None)
        value = name_var.get("value") if name_var else None
        results["locals_name"] = value.strip('"') if isinstance(value, str) else value

        seq = client.send("continue", {"threadId": thread_id})
        results["continue"] = client.wait_response(seq).get("success")
        seq = client.send("disconnect", {})
        results["disconnect"] = client.wait_response(seq).get("success")
    finally:
        bridge.terminate()
        try:
            bridge.wait(timeout=10)
        except subprocess.TimeoutExpired:
            bridge.kill()
        try:
            http("DELETE", base + "/servers/" + server_id, {})
        except Exception:  # noqa: BLE001
            pass
        agent.terminate()
        try:
            agent.wait(timeout=10)
        except subprocess.TimeoutExpired:
            agent.kill()

    print(json.dumps(results, indent=2, ensure_ascii=False))
    expected = {
        "initialize": True,
        "attach": True,
        "stopped": True,
        "continue": True,
        "disconnect": True,
        "locals_name": "Kairo",
        "build": "success",
    }
    failed = [key for key, value in expected.items() if results.get(key) != value]
    print("RESULT: %s" % ("PASS" if not failed else "FAIL: " + ", ".join(failed)))


if __name__ == "__main__":
    main()
