// rust call this function...
export const js_load_asset = (url) => {
    fetch(url)
        .then(data=>{
            return data.arrayBuffer();
        })
        .then(res=>{
            sg.on_asset_loaded(url, new Uint8Array(res));
        })
    ;
};

const utils = {};

utils.loop = update => {
    let lastTime = performance.now();
    const frameDuration = 1000 / 60; 

    const loopFunction = function(currentTime) {
        const deltaTime = currentTime - lastTime;

        if (deltaTime >= frameDuration) {
            const times = Math.floor(deltaTime / frameDuration);
            update((times * frameDuration) * 0.001); // 将毫秒转换为秒
            lastTime += times * frameDuration;
        }

        requestAnimationFrame(loopFunction);
    };

    requestAnimationFrame(loopFunction);
};

import init, {PixelGame} from "./pkg/pixel.js";
const wasm = await init();

const timg = new Image();
timg.src = "assets/pix/symbols.png";
await timg.decode();
const canvas = document.createElement("canvas");
canvas.width = timg.width;
canvas.height = timg.height;
const ctx = canvas.getContext("2d");
ctx.drawImage(timg, 0, 0);
const imgdata = ctx.getImageData(0,0,timg.width,timg.height).data;

const sg = PixelGame.new();
sg.upload_imgdata(timg.width, timg.height, imgdata);

// send event to rust...
window.onkeypress = (e) => { sg.key_event(0, e); };
window.onmouseup = (e) => { sg.key_event(1, e); };
window.onmousedown = (e) => { sg.key_event(2, e); };
window.onmousemove = (e) => { sg.key_event(3, e); };

utils.loop(function(timeStep) {
    sg.tick(timeStep);
    return true;
});

