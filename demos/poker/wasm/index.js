// 初始化wasm
import init, {WasmGinRummy} from "./pkg/poker_wasm.js";
const wasm = await init();
const wgr = WasmGinRummy.new();
console.log("before assign...");
let cards = new Uint16Array([1,40, 2,3,4,5,31,32,33,41]);
wgr.assign(cards, 0);
const wbuflen = wgr.web_buffer_len();
const wbufptr = wgr.web_buffer();
let wbuf = new Uint8Array(wasm.memory.buffer, wbufptr, wbuflen);
console.log("after assign...", wbuf);
window.alert(wbuf);
