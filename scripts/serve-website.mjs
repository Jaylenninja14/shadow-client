// Preview website/ locally: npm run website:serve (http://127.0.0.1:5190; port 8080 is avoided on purpose)
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const [, , dir = "website", port = "5190"] = process.argv;
const root = path.resolve(dir);
const types = { ".html": "text/html; charset=utf-8", ".css": "text/css", ".js": "text/javascript", ".json": "application/json", ".png": "image/png", ".svg": "image/svg+xml", ".exe": "application/octet-stream", ".msi": "application/octet-stream", ".zip": "application/zip" };
http.createServer((req, res) => {
  const url = decodeURIComponent(new URL(req.url, "http://x").pathname);
  let file = path.join(root, url);
  if (!file.startsWith(root)) { res.writeHead(403).end(); return; }
  if (fs.existsSync(file) && fs.statSync(file).isDirectory()) file = path.join(file, "index.html");
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404).end("not found"); return; }
    res.writeHead(200, { "content-type": types[path.extname(file)] ?? "application/octet-stream" }).end(data);
  });
}).listen(Number(port), "127.0.0.1", () => console.log(`serving ${root} on http://127.0.0.1:${port}`));
