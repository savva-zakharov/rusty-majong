// Local test server for the daily challenge leaderboard.
//
//   node web/dev-server.js [port]          (default 8080)
//
// Serves the web build in dist/ (run web/build.sh first) and the leaderboard
// API at /api/daily, using the real handler from the pano-gallery site
// (../pano-gallery/api/daily.js, or set DAILY_API to its path). Instead of
// Upstash Redis it keeps the scores in memory, answering the handful of
// Redis commands the handler uses through a fake REST endpoint, so nothing
// leaves this machine and every restart starts empty.
//
// It listens on the local network too, so phones on the same Wi-Fi can open
// http://<this computer's IP>:8080/ and play the web build against it.
// Desktop builds can use it too:
//   cargo run -- --leaderboard http://localhost:8080/api/daily

const http = require("http");
const fs = require("fs");
const path = require("path");

const port = Number(process.argv[2] || 8080);
const root = path.join(__dirname, "..", "dist");
const api = path.resolve(process.env.DAILY_API || path.join(__dirname, "..", "..", "pano-gallery", "api", "daily.js"));

process.env.KV_REST_API_URL = `http://localhost:${port}/_redis`;
process.env.KV_REST_API_TOKEN = "local";
const handler = require(api);

// ---- fake Redis: sorted sets, hashes and counters, in memory
const zsets = new Map(); // key -> Map(member -> score)
const hashes = new Map();
const counters = new Map();

function sorted(key) {
    const z = zsets.get(key) || new Map();
    return [...z.entries()].sort((a, b) => a[1] - b[1] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
}

function command([name, key, ...args]) {
    switch (name.toUpperCase()) {
        case "ZADD": {
            if (!zsets.has(key)) zsets.set(key, new Map());
            const z = zsets.get(key);
            const added = z.has(args[1]) ? 0 : 1;
            z.set(args[1], Number(args[0]));
            return added;
        }
        case "ZSCORE": {
            const s = zsets.get(key)?.get(args[0]);
            return s === undefined ? null : String(s);
        }
        case "ZCARD":
            return zsets.get(key)?.size || 0;
        case "ZRANK": {
            const i = sorted(key).findIndex(([m]) => m === args[0]);
            return i < 0 ? null : i;
        }
        case "ZRANGE": {
            const [start, stop] = [Number(args[0]), Number(args[1])];
            const out = [];
            for (const [m, s] of sorted(key).slice(start, stop + 1)) out.push(m, String(s));
            return out;
        }
        case "HSET":
            if (!hashes.has(key)) hashes.set(key, new Map());
            hashes.get(key).set(args[0], args[1]);
            return 1;
        case "INCR":
            counters.set(key, (counters.get(key) || 0) + 1);
            return counters.get(key);
        case "EXPIRE":
            return 1;
        default:
            throw new Error(`fake redis: ${name} not supported`);
    }
}

const types = {
    ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
    ".png": "image/png", ".wav": "audio/wav", ".txt": "text/plain",
};

http.createServer(async (req, res) => {
    const url = new URL(req.url, "http://x");
    if (url.pathname === "/_redis/pipeline") {
        let raw = "";
        for await (const c of req) raw += c;
        const out = JSON.parse(raw).map((c) => {
            try {
                return { result: command(c) };
            } catch (e) {
                return { error: e.message };
            }
        });
        res.setHeader("Content-Type", "application/json");
        return res.end(JSON.stringify(out));
    }
    if (url.pathname === "/api/daily") {
        console.log(req.method, req.url);
        return handler(req, res);
    }
    // Static files from dist/ (the game is at /, as well as /mahjong/ like the site).
    const rel = decodeURIComponent(url.pathname).replace(/^\/mahjong/, "");
    const file = path.join(root, rel.endsWith("/") ? rel + "index.html" : rel);
    if (!file.startsWith(root) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
        res.statusCode = 404;
        return res.end("not found");
    }
    res.setHeader("Content-Type", types[path.extname(file)] || "application/octet-stream");
    res.setHeader("Cache-Control", "no-store");
    fs.createReadStream(file).pipe(res);
}).listen(port, () => {
    // Listens on every network interface, so phones on the same network can play too.
    const lan = Object.values(require("os").networkInterfaces()).flat()
        .filter((i) => i.family === "IPv4" && !i.internal)
        .map((i) => `http://${i.address}:${port}/`);
    console.log(`daily challenge test server (API ${api}):`);
    for (const u of [`http://localhost:${port}/`, ...lan]) console.log(`  ${u}`);
});
