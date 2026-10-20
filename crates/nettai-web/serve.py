"""Serves the page build.sh made, as a page with threads must be: cross-origin isolated (COOP and COEP), so that
its memory can be shared with its workers. For development: http://localhost:8000/

    python3 crates/nettai-web/serve.py [folder] [port]
"""
import functools
import http.server
import sys


class Isolated(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


folder = sys.argv[1] if len(sys.argv) > 1 else "target/web/site"
port = int(sys.argv[2]) if len(sys.argv) > 2 else 8000
server = http.server.ThreadingHTTPServer(("127.0.0.1", port), functools.partial(Isolated, directory=folder))
print(f"serving {folder} at http://localhost:{port}/")
server.serve_forever()
