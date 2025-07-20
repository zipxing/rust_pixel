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

// Canvas 自适应函数
const resizeCanvas = () => {
    const canvas = document.getElementById("canvas");
    if (canvas) {
        // WASM应用的实际尺寸
        const actualWidth = 1040;  // 52 * 8
        const actualHeight = 640; // 32 * 8
        
        // 获取父容器尺寸
        const parent = canvas.parentElement;
        const rect = parent.getBoundingClientRect();
        
        // 计算缩放比例，保持宽高比
        const scaleX = rect.width / actualWidth;
        const scaleY = rect.height / actualHeight;
        const scale = Math.min(scaleX, scaleY);
        
        // 计算居中显示的尺寸
        const displayWidth = actualWidth * scale;
        const displayHeight = actualHeight * scale;
        
        // 设置画布显示尺寸和位置（居中）
        canvas.style.width = displayWidth + 'px';
        canvas.style.height = displayHeight + 'px';
        canvas.style.left = ((rect.width - displayWidth) / 2) + 'px';
        canvas.style.top = ((rect.height - displayHeight) / 2) + 'px';
        
        // 设置实际渲染分辨率，匹配WASM应用
        canvas.width = actualWidth;
        canvas.height = actualHeight;
    }
};

import init, {PetviewGame} from "./pkg/petview.js";
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

const sg = PetviewGame.new();
sg.upload_imgdata(timg.width, timg.height, imgdata);

// 初始化canvas尺寸
resizeCanvas();

// 监听窗口尺寸变化
window.addEventListener('resize', resizeCanvas);

// send event to rust...
window.onkeypress = (e) => { sg.key_event(0, e); };
window.onmouseup = (e) => { sg.key_event(1, e); };
window.onmousedown = (e) => { sg.key_event(2, e); };
window.onmousemove = (e) => { sg.key_event(3, e); };

utils.loop(function(timeStep) {
    sg.tick(timeStep);
    return true;
});

