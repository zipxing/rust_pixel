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

import init, {PixelGame} from "./pkg/pixel.js";
const wasm = await init();
const sg = PixelGame.new();

// send event to rust...
window.onkeypress = (e) => { sg.key_event(0, e); };
window.onmouseup = (e) => { sg.key_event(1, e); };
window.onmousedown = (e) => { sg.key_event(2, e); };
window.onmousemove = (e) => { sg.key_event(3, e); };

// creat pix object and sprites...
const pix = new Pix(document.getElementById("canvas"));
const spriteSheet = new pix.Texture("assets/pix/c64.png");
spriteSheet.bind();
pix.setClearColor(new Pix.Color(0.1, 0.1, 0.1));
const drawCells = [];
for (let i=0; i<32; i++) {
    for (let j=0; j<32; j++) {
        let name = "" + (i * 32 + j);
        pix.register(name, pix.makeCellFrame(
                                               spriteSheet,
                                               j*17, i*17, 16, 16,
                                               8, 8, 0));
        drawCells.push(new pix.Cell(name));
    }
}

const transform = new Pix.Transform();
pix.utils.loop(function(timeStep) {
    sg.tick(timeStep);
    const wbufptr = sg.web_buffer();
    const wbuflen = sg.web_buffer_len();
    const wclen = sg.web_cell_len();
    const ratio_x = sg.get_ratiox();
    const ratio_y = sg.get_ratioy();
    let wbuf = new Uint32Array(wasm.memory.buffer, wbufptr, wbuflen * wclen);
    pix.bind();
    pix.clear();
    // draw sprites...
    for (let i = 0; i < wbuflen; ++i) {
        const base = i * wclen;
        const r = wbuf[base + 0];
        const g = wbuf[base + 1];
        const b = wbuf[base + 2];
        const texidx = wbuf[base + 3]; 
        const spx = wbuf[base + 4] + 16;
        const spy = wbuf[base + 5] + 16;
        const ang = wbuf[base + 8] / 1000.0;
        const cpx = wbuf[base + 9] | 0;
        const cpy = wbuf[base + 10] | 0;
        transform.identity();
        transform.translate(spx + cpx - 8, spy + cpy - 8);
        if(ang != 0.0) transform.rotate(ang);
        transform.translate(-cpx + 8, -cpy + 8);
        transform.scale(1.0 / ratio_x, 1.0 / ratio_y);
        drawCells[texidx].draw(transform, r / 255.0, g / 255.0, b / 255.0, 1.0);
    }
    // only 1 draw call...
    pix.flush();
    return true;
});

