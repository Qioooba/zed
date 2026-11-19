"""Real WebSocket /api/v1/events test against a running Kairo agent."""
import base64
import json
import os
import shutil
import socket
import struct
import subprocess
import time
import urllib.request

ROOT = r"D:\GPU-Tools\Temp\opencode\kairo-ws-test"
AGENT = r"G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe"
BUNDLED = r"G:\spaces\kairo-ide\bundled"
SAMPLE = r"G:\spaces\kairo-ide\legacy-sample"


def http(url, payload=None):
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(url, data=data, method="POST" if data else "GET",
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as response:
        return json.load(response)


def ws_connect(port, workspace_id):
    sock = socket.create_connection(("127.0.0.1", port), timeout=10)
    key = base64.b64encode(os.urandom(16)).decode()
    request = (
        "GET /api/v1/events?workspaceId=%s HTTP/1.1\r\n" % workspace_id +
        "Host: 127.0.0.1:%d\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Key: %s\r\n"
        "Sec-WebSocket-Version: 13\r\n"
        "Sec-WebSocket-Protocol: kairo-secret-v1\r\n\r\n" % (port, key)
    )
    sock.sendall(request.encode())
    buffer = b""
    while b"\r\n\r\n" not in buffer:
        buffer += sock.recv(4096)
    header, rest = buffer.split(b"\r\n\r\n", 1)
    if b"101" not in header.split(b"\r\n")[0]:
        raise RuntimeError(header.decode("utf-8", "replace")[:300])
    return sock, rest


def read_frames(sock, remaining, seconds=12):
    events = []
    deadline = time.time() + seconds
    buf = remaining
    while time.time() < deadline:
        sock.settimeout(max(0.2, deadline - time.time()))
        try:
            chunk = sock.recv(65536)
            if not chunk:
                break
            buf += chunk
        except socket.timeout:
            break
        while len(buf) >= 2:
            opcode = buf[0] & 0x0F
            masked = buf[1] & 0x80
            length = buf[1] & 0x7F
            offset = 2
            if length == 126:
                if len(buf) < 4:
                    break
                length = struct.unpack("!H", buf[2:4])[0]
                offset = 4
            elif length == 127:
                if len(buf) < 10:
                    break
                length = struct.unpack("!Q", buf[2:10])[0]
                offset = 10
            mask = b""
            if masked:
                if len(buf) < offset + 4:
                    break
                mask = buf[offset:offset + 4]
                offset += 4
            if len(buf) < offset + length:
                break
            payload = buf[offset:offset + length]
            buf = buf[offset + length:]
            if mask:
                payload = bytes(byte ^ mask[i % 4] for i, byte in enumerate(payload))
            if opcode == 1:
                try:
                    events.append(json.loads(payload.decode("utf-8")))
                except ValueError:
                    pass
    return events


def main():
    shutil.rmtree(ROOT, ignore_errors=True)
    os.makedirs(ROOT, exist_ok=True)
    data_dir = os.path.join(ROOT, "agent")
    os.makedirs(data_dir, exist_ok=True)
    project = os.path.join(ROOT, "legacy-sample")
    shutil.copytree(SAMPLE, project)
    shutil.rmtree(os.path.join(project, ".kairo"), ignore_errors=True)
    agent = subprocess.Popen(
        [AGENT, "-port", "0", "-bind", "127.0.0.1", "-data-dir", data_dir,
         "-bundled-dir", BUNDLED, "-log-level", "warn"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    port = None
    state_path = os.path.join(data_dir, "agent-state.json")
    for _ in range(200):
        time.sleep(0.1)
        if os.path.exists(state_path):
            state = json.load(open(state_path, encoding="utf-8"))
            if state.get("status") == "ready":
                port = state["port"]
                break
    try:
        base = "http://127.0.0.1:%d/api/v1" % port
        ws = http(base + "/workspaces", {"rootPath": project})
        wsid = ws["payload"]["id"]
        sock, remaining = ws_connect(port, wsid)
        # Generate real events while subscribed.
        http(base + "/workspaces/%s/scan" % wsid, {"rootPath": project})
        imp = http(base + "/projects/import", {
            "workspaceId": wsid, "name": "ws-test", "rootPath": project,
            "sourceDirs": ["src/main/java"], "webappDir": "WebRoot",
            "outputDir": "build/classes", "buildFile": "build.xml",
            "sourceLevel": "1.6", "targetLevel": "1.6", "encoding": "gbk",
            "buildTool": "ant", "contextPath": "/"})
        project_id = imp["payload"]["id"]
        http(base + "/builds", {"projectId": project_id})
        events = read_frames(sock, remaining, seconds=12)
        sock.close()
        sequences = [event.get("sequence", 0) for event in events]
        gaps = sum(1 for previous, current in zip(sequences, sequences[1:])
                   if current > previous + 1)
        print("WS_EVENTS: received=%d gaps=%d" % (len(events), gaps))
        for event in events[:8]:
            print("  seq=%s type=%s" % (event.get("sequence"), event.get("type")))
        print("RESULT: %s" % ("PASS" if events else "FAIL"))
    finally:
        agent.terminate()
        try:
            agent.wait(timeout=10)
        except subprocess.TimeoutExpired:
            agent.kill()


if __name__ == "__main__":
    main()
