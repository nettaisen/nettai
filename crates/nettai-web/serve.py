"""Serves the page build.sh made, as a page with threads must be: cross-origin isolated (COOP and COEP), so that
its memory can be shared with its workers. For development: http://localhost:8000/

    python3 crates/nettai-web/serve.py [folder] [port]
"""
import functools
import http.server
import os
import sys


class Isolated(http.server.SimpleHTTPRequestHandler):
    # (Connections kept open: each worker fetches the module's scripts again, all at once.)
    protocol_version = "HTTP/1.1"
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    # The file compressed beside it (build.sh makes them), where the browser takes that encoding.
    ENCODINGS = (("br", ".br"), ("gzip", ".gz"))

    def send_head(self):
        path = self.translate_path(self.path)
        accepted = self.headers.get("Accept-Encoding", "")
        for encoding, suffix in self.ENCODINGS:
            if encoding in accepted and os.path.isfile(path + suffix):
                with open(path + suffix, "rb") as f:
                    body = f.read()
                self.send_response(200)
                self.send_header("Content-Type", self.guess_type(path))
                self.send_header("Content-Encoding", encoding)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                return _Body(body)
        return super().send_head()

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


class _Body:
    """A body already read, as send_head hands one back."""

    def __init__(self, body):
        self.body = body

    def read(self, *_):
        body, self.body = self.body, b""
        return body

    def close(self):
        pass


folder = sys.argv[1] if len(sys.argv) > 1 else "target/web/site"
port = int(sys.argv[2]) if len(sys.argv) > 2 else 8000
class Server(http.server.ThreadingHTTPServer):
    # (The default queue is 5 connections: past that the rest are reset, and a worker's scripts fail to load.)
    request_queue_size = 128


server = Server(("127.0.0.1", port), functools.partial(Isolated, directory=folder))
print(f"serving {folder} at http://localhost:{port}/")
server.serve_forever()
