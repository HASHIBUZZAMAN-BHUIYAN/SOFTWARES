"""
HWP Studio Web Server v2.0
Serves the web-based Korean document editor on all platforms.
Run: python server.py
"""
import os
import io
import sys
import json
import socket
import webbrowser
from pathlib import Path
from threading import Timer

# ── Optional dependencies ──────────────────────────────
try:
    from fastapi import FastAPI, Request
    from fastapi.responses import Response, JSONResponse
    from fastapi.staticfiles import StaticFiles
    import uvicorn
    HAS_FASTAPI = True
except ImportError:
    HAS_FASTAPI = False

try:
    from docx import Document as DocxDocument
    HAS_DOCX = True
except ImportError:
    HAS_DOCX = False

try:
    from reportlab.pdfgen import canvas as rl_canvas
    from reportlab.lib.pagesizes import A4
    HAS_PDF = True
except ImportError:
    HAS_PDF = False

STATIC_DIR = Path(__file__).parent / "static"
PORT = int(os.environ.get("PORT", 8765))


def get_local_ip():
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"


# ── FastAPI app ────────────────────────────────────────
if HAS_FASTAPI:
    app = FastAPI(title="HWP Studio")

    @app.get("/api/ping")
    async def ping():
        return {"status": "ok"}

    @app.post("/api/save")
    async def save_file(request: Request):
        data = await request.json()
        path = data.get("path")
        if not path:
            return JSONResponse({"error": "no path"}, status_code=400)
        try:
            p = Path(path)
            if p.suffix.lower() == ".hwps":
                with open(p, "w", encoding="utf-8") as f:
                    json.dump(data, f, ensure_ascii=False, indent=2)
            else:
                with open(p, "w", encoding="utf-8") as f:
                    f.write(data.get("content", ""))
            return {"status": "saved", "path": str(p)}
        except Exception as e:
            return JSONResponse({"error": str(e)}, status_code=500)

    @app.post("/api/export/pdf")
    async def export_pdf(request: Request):
        if not HAS_PDF:
            return JSONResponse({"error": "reportlab not installed: pip install reportlab"}, status_code=501)
        data = await request.json()
        content = data.get("content", "")
        title = data.get("title", "document")
        buf = io.BytesIO()
        c = rl_canvas.Canvas(buf, pagesize=A4)
        w, h = A4
        y = h - 60
        c.setFont("Helvetica", 11)
        for line in content.split("\n"):
            if y < 60:
                c.showPage()
                y = h - 60
                c.setFont("Helvetica", 11)
            c.drawString(60, y, line[:100])
            y -= 18
        c.save()
        buf.seek(0)
        return Response(
            content=buf.read(),
            media_type="application/pdf",
            headers={"Content-Disposition": f'attachment; filename="{title}.pdf"'},
        )

    @app.post("/api/export/docx")
    async def export_docx(request: Request):
        if not HAS_DOCX:
            return JSONResponse({"error": "python-docx not installed: pip install python-docx"}, status_code=501)
        data = await request.json()
        content = data.get("content", "")
        title = data.get("title", "document")
        doc = DocxDocument()
        for line in content.split("\n"):
            doc.add_paragraph(line)
        buf = io.BytesIO()
        doc.save(buf)
        buf.seek(0)
        return Response(
            content=buf.read(),
            media_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            headers={"Content-Disposition": f'attachment; filename="{title}.docx"'},
        )

    app.mount("/", StaticFiles(directory=str(STATIC_DIR), html=True), name="static")


# ── Entry point ────────────────────────────────────────
def main():
    local_ip = get_local_ip()
    print(f"\n{'═'*54}")
    print(f"  HWP Studio Web v2.0  │  한국어 문서 편집기")
    print(f"{'═'*54}")
    print(f"  PC/Mac   →  http://localhost:{PORT}")
    print(f"  태블릿   →  http://{local_ip}:{PORT}")
    print(f"  (같은 WiFi에 연결된 기기에서 접속)")
    print(f"{'─'*54}")
    print(f"  종료: Ctrl+C")
    print(f"{'═'*54}\n")

    def open_browser():
        webbrowser.open(f"http://localhost:{PORT}")

    if HAS_FASTAPI:
        Timer(1.2, open_browser).start()
        uvicorn.run(app, host="0.0.0.0", port=PORT, log_level="warning")
    else:
        # stdlib fallback (no file-system API, but editor still works)
        import http.server
        os.chdir(STATIC_DIR)

        class Handler(http.server.SimpleHTTPRequestHandler):
            def log_message(self, fmt, *args):
                pass

        Timer(0.5, open_browser).start()
        print("  [주의] fastapi/uvicorn 미설치 – 기본 서버 실행 중")
        print("  pip install fastapi uvicorn 으로 전체 기능 사용 가능\n")
        with http.server.HTTPServer(("0.0.0.0", PORT), Handler) as httpd:
            httpd.serve_forever()


if __name__ == "__main__":
    main()
