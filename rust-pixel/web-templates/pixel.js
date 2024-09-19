const Pix = function(canvasElement, antialias) {
    const _gl = canvasElement.getContext("webgl2", {
        antialias: antialias ? antialias : false,
        depth: false,
        alpha: false
    });

    this.Texture = function() {
        this.free = () => {
            _gl.deleteTexture(_texture);
            _gl.deleteFramebuffer(_framebuffer);
        };

        this.bind = () => {
            console.log("texture bind....");
            bind(this);
            _gl.bindFramebuffer(_gl.FRAMEBUFFER, _framebuffer);
            _gl.viewport(0, 0, _width, _height);
        };

        this._addFrame = frame => {
            _frames.push(frame);
        };

        this._getTexture = () => _texture;
        this.getWidth = () => _width;
        this.getHeight = () => _height;
        this.setClearColor = color => _clearColor = color;
        this.clear = () => clear(_clearColor);
        this.ready = () => _ready;

        const _texture = _gl.createTexture();
        const _framebuffer = _gl.createFramebuffer();
        const _frames = [];

        let _ready = false;
        let _width = 0;
        let _height = 0;
        let _clearColor = new Pix.Color(1, 1, 1, 0);

        _gl.activeTexture(TEXTURE_EDITING);
        _gl.bindTexture(_gl.TEXTURE_2D, _texture);

        const image = new Image();

        image.onload = () => {
            if(_width === 0 || _height === 0) {
                _width = image.width;
                _height = image.height;
            }
            _gl.activeTexture(TEXTURE_EDITING);
            _gl.bindTexture(_gl.TEXTURE_2D, _texture);
            _gl.texImage2D(_gl.TEXTURE_2D, 0, _gl.RGBA, _gl.RGBA, _gl.UNSIGNED_BYTE, image);
            for(let frame = _frames.pop(); frame !== undefined; frame = _frames.pop()) {
                frame[5] /= _width;
                frame[6] /= _height;
                frame[7] /= _width;
                frame[8] /= _height;
            }
            _ready = true;
        };

        const source = arguments[0];

        image.crossOrigin = "Anonymous";
        image.src = source;

        _gl.texImage2D(_gl.TEXTURE_2D, 0, _gl.RGBA, 1, 1, 0, _gl.RGBA, _gl.UNSIGNED_BYTE, _emptyPixel);

        _gl.texParameteri(_gl.TEXTURE_2D, _gl.TEXTURE_MAG_FILTER, _gl.NEAREST);
        _gl.texParameteri(_gl.TEXTURE_2D, _gl.TEXTURE_MIN_FILTER, _gl.NEAREST);

        _gl.texParameteri(_gl.TEXTURE_2D, _gl.TEXTURE_WRAP_S, _gl.CLAMP_TO_EDGE);
        _gl.texParameteri(_gl.TEXTURE_2D, _gl.TEXTURE_WRAP_T, _gl.CLAMP_TO_EDGE);

        {
            const previousFramebuffer = _gl.getParameter(_gl.FRAMEBUFFER_BINDING);
            _gl.bindFramebuffer(_gl.FRAMEBUFFER, _framebuffer);
            _gl.framebufferTexture2D(_gl.FRAMEBUFFER, _gl.COLOR_ATTACHMENT0, _gl.TEXTURE_2D, _texture, 0);
            _gl.bindFramebuffer(_gl.FRAMEBUFFER, previousFramebuffer);
        }
    };

    this.Cell = function(name) {
        this.draw = (t, r, g, b, a) => {
            const frame = this._getFrame();
            bindTextureAtlas(frame[0]);
            prepareDraw(RENDER_MODE_PIXCELLS, 16);
            _instanceBuffer[++_instanceBufferAt] = frame[3];
            _instanceBuffer[++_instanceBufferAt] = frame[4];
 
            // uv attrs...
            _instanceBuffer[++_instanceBufferAt] = frame[5];
            _instanceBuffer[++_instanceBufferAt] = frame[6];
            _instanceBuffer[++_instanceBufferAt] = frame[7];
            _instanceBuffer[++_instanceBufferAt] = frame[8];

            // transform attrs...
            _instanceBuffer[++_instanceBufferAt] = t._00 * frame[1];
            _instanceBuffer[++_instanceBufferAt] = t._10 * frame[2];
            _instanceBuffer[++_instanceBufferAt] = t._01 * frame[1];
            _instanceBuffer[++_instanceBufferAt] = t._11 * frame[2];
            _instanceBuffer[++_instanceBufferAt] = t._20;
            _instanceBuffer[++_instanceBufferAt] = t._21;

            // color...
            _instanceBuffer[++_instanceBufferAt] = r; 
            _instanceBuffer[++_instanceBufferAt] = g;
            _instanceBuffer[++_instanceBufferAt] = b;
            _instanceBuffer[++_instanceBufferAt] = a;
        };

        this._getFrame = () => _frames[_frame];
        this.setFrame = index => _frame = index;
        this.getFrame = () => _frame;
        this.getFrameCount = () => _frames.length;

        const _frames = _sprites[name];
        let _frameCounter = 0;
        let _frame = 0;
    };

    this.utils = {};

    this.utils.loop = update => {
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

    const ShaderCore = function(vertex, fragment) {
        const createShader = (type, source) => {
            const shader = _gl.createShader(type);

            _gl.shaderSource(shader, "#version 300 es\n" + source);
            _gl.compileShader(shader);

            if(!_gl.getShaderParameter(shader, _gl.COMPILE_STATUS))
                console.log(_gl.getShaderInfoLog(shader));

            return shader;
        };

        this.bind = () => {
            if(_currentShaderCore === this)
                return;

            _currentShaderCore = this;

            _gl.useProgram(_program);
        };

        this.getProgram = () => _program;
        this.free = () => _gl.deleteProgram(_program);
        this.getVertex = () => vertex;
        this.getFragment = () => fragment;

        const _program = _gl.createProgram();
        const _shaderVertex = createShader(_gl.VERTEX_SHADER, vertex);
        const _shaderFragment = createShader(_gl.FRAGMENT_SHADER, fragment);

        _gl.attachShader(_program, _shaderVertex);
        _gl.attachShader(_program, _shaderFragment);
        _gl.linkProgram(_program);
        _gl.detachShader(_program, _shaderVertex);
        _gl.detachShader(_program, _shaderFragment);
        _gl.deleteShader(_shaderVertex);
        _gl.deleteShader(_shaderFragment);
    };

    const Shader = function(core, uniforms) {
        this.bind = () => {
            if(_currentShader === this) {
                for (const uniformCall of _uniformCalls)
                    uniformCall[0](uniformCall[1], uniformCall[2].value);

                return;
            }

            _currentShader = this;

            core.bind();

            for (const uniformCall of _uniformCalls)
                uniformCall[0](uniformCall[1], uniformCall[2].value);
        };

        this.setUniform = (name, value) => uniforms[name].value = value;
        this.free = () => core.free();

        const _uniformCalls = [];

        for (const uniform of Object.keys(uniforms))
            _uniformCalls.push([
                _gl["uniform" + uniforms[uniform].type].bind(_gl),
                _gl.getUniformLocation(core.getProgram(), uniform),
                uniforms[uniform]
            ]);
    };

    const bind = target => {
        if(_surface === target)
            return;

        flush();

        if(_surface != null) {
            console.log("11111", _surface);
            this.pop();
        }

        if(target != null) {
            console.log("22222", target);
            pushIdentity();
        }

        _surface = target;
        console.log("33333", _surface, target);
    };

    const bindTextureTexture = texture => {
        if(_currentTextureTexture === texture)
            return;

        flush();

        _gl.activeTexture(TEXTURE_SURFACE);
        _gl.bindTexture(_gl.TEXTURE_2D, texture);

        _currentTextureTexture = texture;
    };

    const bindTextureAtlas = texture => {
        if(_currentTextureAtlas === texture)
            return;

        flush();

        _gl.activeTexture(TEXTURE_ATLAS);
        _gl.bindTexture(_gl.TEXTURE_2D, texture);

        _currentTextureAtlas = texture;
    };

    const clear = color => {
        flush();

        _gl.clearColor(color.r * _uboContents[8], color.g * _uboContents[9], color.b * _uboContents[10], color.a * _uboContents[11]);
        _gl.clear(_gl.COLOR_BUFFER_BIT);
    };

    const flush = this.flush = () => {
        if(_instanceCount === 0)
            return;

        _gl.bindBuffer(_gl.ARRAY_BUFFER, _instances);
        _gl.bufferSubData(_gl.ARRAY_BUFFER, 0, _instanceBuffer, 0, _instanceBufferAt + 1);

        // switch(_renderMode) {
        //     case RENDER_MODE_PIXCELLS:
                _gl.bindVertexArray(_vaoCells);
                _gl.drawArraysInstanced(_gl.TRIANGLE_FAN, 0, 4, _instanceCount);
        //        break;
        // }

        _instanceBufferAt = -1;
        _instanceCount = 0;
    };

    const sendUniformBuffer = () => {
        if(_surface == null) {
            _uboContents[3] = canvasElement.width;
            _uboContents[7] = canvasElement.height;
        }
        else {
            _uboContents[3] = _surface.getWidth();
            _uboContents[7] = _surface.getHeight();
        }

        _uboContents[0] = _transformStack[_transformAt]._00;
        _uboContents[1] = _transformStack[_transformAt]._10;
        _uboContents[2] = _transformStack[_transformAt]._20;
        _uboContents[4] = _transformStack[_transformAt]._01;
        _uboContents[5] = _transformStack[_transformAt]._11;
        _uboContents[6] = _transformStack[_transformAt]._21;

        _gl.bufferSubData(_gl.UNIFORM_BUFFER, 0, _uboContents);

        _transformDirty = false;
    };

    const prepareDraw = (mode, size, shader) => {
        if(_transformDirty) {
            flush();

            sendUniformBuffer();
        }

        if(_renderMode !== mode) {
            flush();

            _renderMode = mode;
            (shader || _shaders[mode]).bind();
        }

        if(_instanceBufferAt + size >= _instanceBufferCapacity) {
            const oldBuffer = _instanceBuffer;

            _instanceBuffer = new Float32Array(_instanceBufferCapacity *= 2);

            _gl.bindBuffer(_gl.ARRAY_BUFFER, _instances);
            _gl.bufferData(_gl.ARRAY_BUFFER, _instanceBufferCapacity * 4, _gl.DYNAMIC_DRAW);

            for(let i = 0; i < oldBuffer.byteLength; ++i)
                _instanceBuffer[i] = oldBuffer[i];
        }

        ++_instanceCount;
    };

    const pushIdentity = () => {
        if(++_transformAt === _transformStack.length)
            _transformStack.push(new Pix.Transform());
        else
            _transformStack[_transformAt].identity();

        _transformDirty = true;
    };

    this.push = () => {
        if(++_transformAt === _transformStack.length)
            _transformStack.push(_transformStack[_transformAt - 1].copy());
        else
            _transformStack[_transformAt].set(_transformStack[_transformAt - 1]);
    };

    this.pop = () => {
        --_transformAt;

        _transformDirty = true;
    };

    this.bind = () => {
        bind(null);

        _gl.bindFramebuffer(_gl.FRAMEBUFFER, null);
        _gl.viewport(0, 0, canvasElement.width, canvasElement.height);
    };

    this.register = function() {
        const frames = [];

        for(let i = 1; i < arguments.length; ++i)
            frames.push(arguments[i]);

        if(_sprites[arguments[0]] === undefined)
            _sprites[arguments[0]] = frames;
        else {
            _sprites[arguments[0]].length = 0;

            for(let i = 0; i < frames.length; ++i)
                _sprites[arguments[0]].push(frames[i]);
        }
    };

    this.isRegistered = name => _sprites[name] !== undefined;

    this.makeCellFrame = (sheet, x, y, width, height, xOrigin, yOrigin, time) => {
        const frame = [
            sheet._getTexture(),
            width,
            height,
            xOrigin / width,
            yOrigin / height,
            x,
            y,
            width,
            height,
            time
        ];

        sheet._addFrame(frame);

        return frame;
    };

    this.free = () => {
        for(let i = 0; i < _shaders.length; ++i)
            _shaders[i].free();

        _gl.deleteVertexArray(_vaoCells);
        _gl.deleteVertexArray(_vaoLines);
        _gl.deleteVertexArray(_vaoMesh);
        _gl.deleteBuffer(_quad);
        _gl.deleteBuffer(_instances);
        _gl.deleteBuffer(_ubo);
    };

    const touchTransform = () => {
        _transformDirty = true;

        return _transformStack[_transformAt];
    };

    this.getTransform = () => _transformStack[_transformAt];
    this.transformSet = transform => {
        touchTransform().set(_transformStack[0]);
        touchTransform().multiply(transform);
    }
    this.transform = transform => touchTransform().multiply(transform);
    this.translate = (x, y) => touchTransform().translate(x, y);
    this.rotate = angle => touchTransform().rotate(angle);
    this.shear = (x, y) => touchTransform().shear(x, y);
    this.scale = (x, y) => touchTransform().scale(x, y);
    this.setClearColor = color => _clearColor = color;
    this.clear = () => clear(_clearColor);
    this.getWidth = () => canvasElement.width;
    this.getHeight = () => canvasElement.height;
    this.unregister = name => delete _sprites[name];

    const RENDER_MODE_NONE = -1;
    // const RENDER_MODE_SURFACES = 0;
    const RENDER_MODE_PIXCELLS = 1;
    const TEXTURE_ATLAS = _gl.TEXTURE0;
    const TEXTURE_SURFACE = _gl.TEXTURE1;
    const TEXTURE_MESH = _gl.TEXTURE2;
    const TEXTURE_EDITING = _gl.TEXTURE3;
    const TEXTURE_SHADER_FIRST = _gl.TEXTURE4;

    const _quad = _gl.createBuffer();
    const _instances = _gl.createBuffer();
    const _vaoCells = _gl.createVertexArray();
    const _vaoLines = _gl.createVertexArray();
    const _vaoMesh = _gl.createVertexArray();
    const _ubo = _gl.createBuffer();
    const _uboContents = new Float32Array(12);
    const _emptyPixel = new Uint8Array(4);
    const _sprites = [];
    const _transformStack = [new Pix.Transform(1, 0, 0, 0, -1, canvasElement.height)];
    const _uniformBlock = "layout(std140) uniform transform {mediump vec4 tw;mediump vec4 th;lowp vec4 colorFilter;};";
    const _shaderCoreCells = new ShaderCore(
        "layout(location=0) in mediump vec2 vertex;" +
        "layout(location=1) in mediump vec4 a1;" +
        "layout(location=2) in mediump vec4 a2;" +
        "layout(location=3) in mediump vec4 a3;" +
        "layout(location=4) in mediump vec4 color;" +
        _uniformBlock +
        "out mediump vec2 uv;" +
        "out lowp vec4 colorj;" +
        "void main() {" +
        "uv=a1.zw+vertex*a2.xy;" +
        "mediump vec2 transformed=(((vertex-a1.xy)*" +
        "mat2(a2.zw,a3.xy)+a3.zw)*" +
        "mat2(tw.xy,th.xy)+vec2(tw.z,th.z))/" +
        "vec2(tw.w,th.w)*2.0;" +
        "gl_Position=vec4(transformed-vec2(1),0,1);" +
        "colorj=color*colorFilter;" +
        "}",
        "uniform sampler2D source;" +
        _uniformBlock +
        "in mediump vec2 uv;" +
        "in lowp vec4 colorj;" +
        "layout(location=0) out lowp vec4 color;" +
        "void main() {" +
        "color=texture(source,uv)*colorj;" +
        "}"
    );
    const _shaders = [
        new Shader(
            _shaderCoreCells,
            {
                source: {
                    type: "1i",
                    value: 1
                }
            }),
        new Shader(
            _shaderCoreCells,
            {
                source: {
                    type: "1i",
                    value: 0
                }
            })
    ];

    let _currentShader, _currentShaderCore, _surface, _currentTextureTexture, _currentTextureAtlas, _currentTextureMesh;
    let _meshUvLeft, _meshUvTop, _meshUvWidth, _meshUvHeight;
    let _transformAt = 0;
    let _transformDirty = true;
    let _renderMode = RENDER_MODE_NONE;
    let _instanceBufferCapacity = 1024;
    let _instanceBufferAt = -1;
    let _instanceBuffer = new Float32Array(_instanceBufferCapacity);
    let _instanceCount = 0;
    let _clearColor = new Pix.Color(1, 1, 1, 0);

    _uboContents[8] = _uboContents[9] = _uboContents[10] = _uboContents[11] = 1;

    _gl.enable(_gl.BLEND);
    _gl.disable(_gl.DEPTH_TEST);
    _gl.blendFuncSeparate(_gl.SRC_ALPHA, _gl.ONE_MINUS_SRC_ALPHA, _gl.ONE, _gl.ONE_MINUS_SRC_ALPHA);
    _gl.getExtension("EXT_color_buffer_float");

    _gl.bindBuffer(_gl.ARRAY_BUFFER, _instances);
    _gl.bufferData(_gl.ARRAY_BUFFER, _instanceBufferCapacity * 4, _gl.DYNAMIC_DRAW);

    _gl.bindBuffer(_gl.ARRAY_BUFFER, _quad);
    _gl.bufferData(_gl.ARRAY_BUFFER, new Float32Array([0, 0, 0, 1, 1, 1, 1, 0]), _gl.STATIC_DRAW);

    _gl.bindBuffer(_gl.UNIFORM_BUFFER, _ubo);
    _gl.bufferData(_gl.UNIFORM_BUFFER, 48, _gl.DYNAMIC_DRAW);
    _gl.bindBufferBase(_gl.UNIFORM_BUFFER, 0, _ubo);

    _gl.bindVertexArray(_vaoCells);
    _gl.bindBuffer(_gl.ARRAY_BUFFER, _quad);
    _gl.enableVertexAttribArray(0);
    _gl.vertexAttribPointer(0, 2, _gl.FLOAT, false, 8, 0);
    _gl.bindBuffer(_gl.ARRAY_BUFFER, _instances);
    _gl.enableVertexAttribArray(1);
    _gl.vertexAttribDivisor(1, 1);
    _gl.vertexAttribPointer(1, 4, _gl.FLOAT, false, 64, 0);
    _gl.enableVertexAttribArray(2);
    _gl.vertexAttribDivisor(2, 1);
    _gl.vertexAttribPointer(2, 4, _gl.FLOAT, false, 64, 16);
    _gl.enableVertexAttribArray(3);
    _gl.vertexAttribDivisor(3, 1);
    _gl.vertexAttribPointer(3, 4, _gl.FLOAT, false, 64, 32);
    _gl.enableVertexAttribArray(4);
    _gl.vertexAttribDivisor(4, 1);
    _gl.vertexAttribPointer(4, 4, _gl.FLOAT, false, 64, 48);

    _gl.bindVertexArray(null);

    console.log("call bind 111111111", _surface);
    this.bind();
};

Pix.Color = function(r, g, b, a) {
    this.r = r;
    this.g = g;
    this.b = b;
    this.a = a === undefined?1:a;
};

Pix.Color.prototype.copy = function() {
    return new Pix.Color(this.r, this.g, this.b, this.a);
};

Pix.Color.prototype.add = function(color) {
    this.r = Math.min(this.r + color.r, 1);
    this.g = Math.min(this.g + color.g, 1);
    this.b = Math.min(this.b + color.b, 1);

    return this;
};

Pix.Color.prototype.multiply = function(color) {
    this.r *= color.r;
    this.g *= color.g;
    this.b *= color.b;

    return this;
};

Pix.Color.prototype.equals = function(color) {
    return this.r === color.r && this.g === color.g && this.b === color.b && this.a === color.a;
};

Pix.Transform = function(_00, _10, _20, _01, _11, _21) {
    if(_00 === undefined)
        this.identity();
    else {
        this._00 = _00;
        this._10 = _10;
        this._20 = _20;
        this._01 = _01;
        this._11 = _11;
        this._21 = _21;
    }
};

Pix.Transform.prototype.apply = function(vector) {
    const x = vector.x;
    const y = vector.y;

    vector.x = this._00 * x + this._10 * y + this._20;
    vector.y = this._01 * x + this._11 * y + this._21;
};

Pix.Transform.prototype.copy = function() {
    return new Pix.Transform(this._00, this._10, this._20, this._01, this._11, this._21);
};

Pix.Transform.prototype.identity = function() {
    this._00 = 1;
    this._10 = 0;
    this._20 = 0;
    this._01 = 0;
    this._11 = 1;
    this._21 = 0;
};

Pix.Transform.prototype.set = function(transform) {
    this._00 = transform._00;
    this._10 = transform._10;
    this._20 = transform._20;
    this._01 = transform._01;
    this._11 = transform._11;
    this._21 = transform._21;
};

Pix.Transform.prototype.multiply = function(transform) {
    const _00 = this._00;
    const _10 = this._10;
    const _01 = this._01;
    const _11 = this._11;

    this._00 = _00 * transform._00 + _10 * transform._01;
    this._10 = _00 * transform._10 + _10 * transform._11;
    this._20 += _00 * transform._20 + _10 * transform._21;
    this._01 = _01 * transform._00 + _11 * transform._01;
    this._11 = _01 * transform._10 + _11 * transform._11;
    this._21 += _01 * transform._20 + _11 * transform._21;
};

Pix.Transform.prototype.rotate = function(angle) {
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);

    const _00 = this._00;
    const _01 = this._01;

    this._00 = _00 * cos - this._10 * sin;
    this._10 = _00 * sin + this._10 * cos;
    this._01 = _01 * cos - this._11 * sin;
    this._11 = _01 * sin + this._11 * cos;
};

Pix.Transform.prototype.shear = function(x, y) {
    const _00 = this._00;
    const _01 = this._01;

    this._00 += this._10 * y;
    this._10 += _00 * x;
    this._01 += this._11 * y;
    this._11 += _01 * x;
};

Pix.Transform.prototype.translate = function(x, y) {
    this._20 += this._00 * x + this._10 * y;
    this._21 += this._01 * x + this._11 * y;
};

Pix.Transform.prototype.scale = function(x, y) {
    this._00 *= x;
    this._10 *= y;
    this._01 *= x;
    this._11 *= y;
};

Pix.Transform.prototype.invert = function() {
    const s11 = this._00;
    const s02 = this._10 * this._21 - this._11 * this._20;
    const s12 = -this._00 * this._21 + this._01 * this._20;

    const d = 1.0 / (this._00 * this._11 - this._10 * this._01);

    this._00 = this._11 * d;
    this._10 = -this._10 * d;
    this._20 = s02 * d;
    this._01 = -this._01 * d;
    this._11 = s11 * d;
    this._21 = s12 * d;
};

if(typeof module !== 'undefined') module.exports = Pix;
