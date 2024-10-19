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
    let lastDate = new Date();
    const loopFunction = function(step) {
        const date = new Date();
        const delta = date - lastDate;
        if (delta > 0.0) {
            if(update((date - lastDate) * 0.001)){}
            lastDate = date;
        }
        requestAnimationFrame(loopFunction);
    };

    requestAnimationFrame(loopFunction);
};

import init, {PixelGame} from "./pkg/pixel.js";
const wasm = await init();

const timg = new Image();
timg.src = "assets/pix/c64.png";
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

