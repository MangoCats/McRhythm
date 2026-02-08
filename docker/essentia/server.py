"""Essentia HTTP wrapper for WKMP musical flavor extraction.

Exposes essentia_streaming_extractor_music as an HTTP API so wkmp-ai
can call it from any platform (Windows via Docker Desktop, Linux native).

Endpoints:
    GET  /health  - Container health check
    POST /analyze - Analyze audio file, return Essentia JSON output
"""

import json
import os
import subprocess
import tempfile
from http.server import HTTPServer, BaseHTTPRequestHandler

ESSENTIA_BIN = "essentia_streaming_extractor_music"
PORT = 5780


class EssentiaHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self._respond(200, {"status": "ok"})
        else:
            self._respond(404, {"error": "not found"})

    def do_POST(self):
        if self.path != "/analyze":
            self._respond(404, {"error": "not found"})
            return

        # Read request body
        content_length = int(self.headers.get("Content-Length", 0))
        if content_length == 0:
            self._respond(400, {"error": "empty request body"})
            return

        body = self.rfile.read(content_length)
        try:
            request = json.loads(body)
        except json.JSONDecodeError as e:
            self._respond(400, {"error": f"invalid JSON: {e}"})
            return

        file_path = request.get("file_path")
        if not file_path:
            self._respond(400, {"error": "missing file_path"})
            return

        if not os.path.isfile(file_path):
            self._respond(404, {"error": f"file not found: {file_path}"})
            return

        # Run Essentia analysis
        try:
            with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as tmp:
                output_path = tmp.name

            result = subprocess.run(
                [ESSENTIA_BIN, file_path, output_path],
                capture_output=True,
                text=True,
                timeout=300,  # 5 minute timeout per file
            )

            if result.returncode != 0:
                self._respond(500, {
                    "error": "essentia analysis failed",
                    "stderr": result.stderr[:1000],
                    "exit_code": result.returncode,
                })
                return

            with open(output_path, "r") as f:
                essentia_output = json.load(f)

            self._respond(200, essentia_output)

        except subprocess.TimeoutExpired:
            self._respond(504, {"error": "analysis timed out (300s)"})
        except Exception as e:
            self._respond(500, {"error": str(e)})
        finally:
            if os.path.exists(output_path):
                os.unlink(output_path)

    def _respond(self, status, data):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def log_message(self, format, *args):
        # Structured logging
        print(f"[essentia] {args[0]}", flush=True)


if __name__ == "__main__":
    server = HTTPServer(("0.0.0.0", PORT), EssentiaHandler)
    print(f"[essentia] Listening on port {PORT}", flush=True)
    server.serve_forever()
