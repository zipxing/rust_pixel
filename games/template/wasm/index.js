// 初始化wasm
import init, {WasmTemplate} from "./pkg/template_wasm.js";
const wasm = await init();
const wgr = WasmTemplate.new();
console.log("before shuffle...");
wgr.shuffle();
wgr.next();
const wbuflen = wgr.web_buffer_len();
const wbufptr = wgr.web_buffer();
let wbuf = new Uint8Array(wasm.memory.buffer, wbufptr, wbuflen);
console.log("after assign...", wbuf);
window.alert(wbuf);
