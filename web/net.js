"use strict";
// Browser services for the wasm code: HTTP for the daily challenge
// leaderboard (see src/net.rs), which starts a fetch and keeps the answer
// until the game polls for it, and the saved settings in localStorage (see
// src/settings.rs).
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

// ---- settings: one localStorage entry of key=value lines
const settings_key = "rusty-majong-settings";
let settings_bytes = null;

function settings_len() {
    let text = null;
    try { text = localStorage.getItem(settings_key); } catch (e) { /* storage blocked */ }
    settings_bytes = text === null ? null : new TextEncoder().encode(text);
    return settings_bytes ? settings_bytes.length : -1;
}

function settings_read(dest) {
    if (settings_bytes) new Uint8Array(wasm_memory.buffer, dest, settings_bytes.length).set(settings_bytes);
    settings_bytes = null;
}

function settings_write(ptr, len) {
    try { localStorage.setItem(settings_key, net_string(ptr, len)); } catch (e) { /* storage blocked */ }
}

function net_register_plugin(importObject) {
    importObject.env.net_request = net_request;
    importObject.env.net_status = net_status;
    importObject.env.net_len = net_len;
    importObject.env.net_read = net_read;
    importObject.env.settings_len = settings_len;
    importObject.env.settings_read = settings_read;
    importObject.env.settings_write = settings_write;
}

// (named apart from audio.js's register_plugin: both are globals.)
miniquad_add_plugin({ register_plugin: net_register_plugin, version: 2, name: "rusty_net" });
