"use strict";
// HTTP for the game's daily challenge leaderboard (see src/net.rs): starts a
// fetch for the wasm code and keeps the answer until the game polls for it.
// Load after gl.js and before load(), like audio.js.

const net_requests = new Map();
let net_next_id = 1;

function net_string(ptr, len) {
    return new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, ptr, len).slice());
}

function net_request(post, url_ptr, url_len, body_ptr, body_len) {
    const id = net_next_id++;
    const r = { status: 0, bytes: new Uint8Array(0) };
    net_requests.set(id, r);
    const options = { method: post ? "POST" : "GET", cache: "no-store" };
    if (post) {
        options.body = net_string(body_ptr, body_len);
        options.headers = { "Content-Type": "application/json" };
    }
    fetch(net_string(url_ptr, url_len), options)
        .then((res) => res.arrayBuffer().then((b) => {
            r.bytes = new Uint8Array(b);
            r.status = res.status;
        }))
        .catch(() => { r.status = -1; });
    return id;
}

function net_status(id) {
    const r = net_requests.get(id);
    return r ? r.status : -1;
}

function net_len(id) {
    const r = net_requests.get(id);
    return r ? r.bytes.length : 0;
}

function net_read(id, dest) {
    const r = net_requests.get(id);
    if (r && dest) new Uint8Array(wasm_memory.buffer, dest, r.bytes.length).set(r.bytes);
    net_requests.delete(id);
}

function net_register_plugin(importObject) {
    importObject.env.net_request = net_request;
    importObject.env.net_status = net_status;
    importObject.env.net_len = net_len;
    importObject.env.net_read = net_read;
}

// (named apart from audio.js's register_plugin: both are globals.)
miniquad_add_plugin({ register_plugin: net_register_plugin, version: 1, name: "rusty_net" });
