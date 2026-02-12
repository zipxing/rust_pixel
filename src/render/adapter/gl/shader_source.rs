pub const VERTEX_SRC_SYMBOLS: &str = r#"
            precision highp float;
            layout(location=0) in vec2 vertex;
            layout(location=1) in vec4 a1;
            layout(location=2) in vec4 a2;
            layout(location=3) in vec4 a3;
            layout(location=4) in vec4 color;
            layout(std140) uniform transform {
                vec4 tw;
                vec4 th;
                vec4 colorFilter;
            };
            out vec2 uv;
            out vec4 colorj;
            flat out float v_bold;
            void main() {
                v_bold = a1.x < 0.0 ? 1.0 : 0.0;
                vec2 origin = abs(a1.xy);
                uv = a1.zw + vertex * a2.xy;
                vec2 transformed = ((mat2(tw.xy, th.xy) * (mat2(a2.zw, a3.xy) * (vertex - origin) + a3.zw) + vec2(tw.z, th.z))) / vec2(tw.w, th.w) * 2.0;
                vec2 ndc_pos = transformed - vec2(1.0, 1.0);
                // Flip Y to match WGPU coordinate system
                gl_Position = vec4(ndc_pos.x, -ndc_pos.y, 0.0, 1.0);
                colorj = color * colorFilter;
            }
        "#;

pub const FRAGMENT_SRC_SYMBOLS: &str = r#"
            precision highp float;
            uniform sampler2D source;
            layout(std140) uniform transform {
                vec4 tw;
                vec4 th;
                vec4 colorFilter;
            };
            in vec2 uv;
            in vec4 colorj;
            flat in float v_bold;
            layout(location=0) out vec4 color;
            void main() {
                vec4 texColor = texture(source, uv);
                if (v_bold > 0.5) {
                    ivec2 ts = textureSize(source, 0);
                    float dx = 0.35 / float(ts.x);
                    texColor = max(texColor, texture(source, uv + vec2(dx, 0.0)));
                    texColor = max(texColor, texture(source, uv + vec2(-dx, 0.0)));
                    texColor.a = smoothstep(0.15, 0.95, texColor.a);
                }
                color = texColor * colorj;
            }
        "#;

// trans shader ...
pub const VERTEX_SRC_TRANS: &str = r#"
            precision highp float;
            layout(location = 0) in vec2 aPos;
            layout(location = 1) in vec2 aTexCoord;
            out vec2 TexCoord;
            void main() {
                gl_Position = vec4(aPos, 0.0, 1.0);
                TexCoord = aTexCoord;
            }
        "#;

pub const TRANS_FS: [&str; 7] = [
    r#"
          const ivec2 squaresMin = ivec2(20);
          const int steps = 50;
          vec4 transition(vec2 uv) {
            float d = min(progress, 1.0 - progress);
            float dist = steps>0 ? ceil(d * float(steps)) / float(steps) : d;
            vec2 squareSize = 2.0 * dist / vec2(squaresMin);
            vec2 p = dist>0.0 ? (floor(uv / squareSize) + 0.5) * squareSize : uv;
            return mix(getFromColor(p), getToColor(p), progress);
          }
    "#,
    r#"
          float inHeart (vec2 p, vec2 center, float size) {
          if (size==0.0) return 0.0;
          vec2 o = (p-center)/(1.6*size);
          float a = o.x*o.x+o.y*o.y-0.3;
          return step(a*a*a, o.x*o.x*o.y*o.y*o.y);
          }
          vec4 transition (vec2 uv) {
          return mix(
          getFromColor(uv),
          getToColor(uv),
          inHeart(uv, vec2(0.5, 0.4), progress)
          );
          }
    "#,
    r#"
            const float GRID = 16.0;
            const float RADIUS = 0.5;

            highp float hash(vec2 co)
            {
                highp float dt = dot(co, vec2(12.9898, 78.233));
                return fract(sin(mod(dt, 3.14)) * 43758.5453);
            }
            vec4 transition(vec2 p) {
              vec2 cell = floor(p * GRID);
              vec2 local = fract(p * GRID) - 0.5;
              float dist = length(local);
              float n = hash(cell);
              float grow = smoothstep(n - 0.1, n + 0.1, progress * 1.2 - 0.1);
              float circle = 1.0 - smoothstep(RADIUS * grow - 0.05, RADIUS * grow, dist);
              return mix(getFromColor(p), getToColor(p), circle);
            }
    "#,
    r#"
            vec4 transition(vec2 uv) {
              float phase = progress < 0.5 ? progress * 2.0 : (progress - 0.5) * 2.0;
              float angleOffset = progress < 0.5 ? mix(0.0, 6.0 * 0.03927, phase) : mix(-6.0 * 0.03927, 0.0, phase);
              float newScale = progress < 0.5 ? mix(1.0, 1.2, phase) : mix(1.2, 1.0, phase);
              vec2 center = vec2(0, 0);
              vec2 assumedCenter = vec2(0.5, 0.5);
              vec2 p = (uv.xy - vec2(0.5, 0.5)) / newScale * vec2(1.2, 1.0);
              float angle = atan(p.y, p.x) + angleOffset;
              float dist = distance(center, p);
              p.x = cos(angle) * dist / 1.2 + 0.5;
              p.y = sin(angle) * dist + 0.5;
              vec4 c = progress < 0.5 ? getFromColor(p) : getToColor(p);
              return c + (progress < 0.5 ? mix(0.0, 1.0, phase) : mix(1.0, 0.0, phase));
              }
    "#,
    r#"
            vec4 transition (vec2 uv) {
                    float time = progress;
                    float stime = sin(time * 3.14159265 / 2.);
                    float phase = time * 3.14159265 * 3.0;
                    float y = (abs(cos(phase))) * (1.0 - stime);
                    float d = uv.y - y;
                    vec4 from = getFromColor(vec2(uv.x, uv.y + (1.0 - y)));
                    // vec4 from = getFromColor(uv);
                    vec4 to = getToColor(uv);
                    vec4 mc = mix( to, from, step(d, 0.0) );
                    return mc;
            }
    "#,
    r#"
            const float size = 0.04;
            const float zoom = 50.0;
            const float colorSeparation = 0.3;

            vec4 transition(vec2 p) {
              float inv = 1. - progress;
              vec2 disp = size*vec2(cos(zoom*p.x), sin(zoom*p.y));
              vec4 texTo = getToColor(p + inv*disp);
              vec4 texFrom = vec4(
                getFromColor(p + progress*disp*(1.0 - colorSeparation)).r,
                getFromColor(p + progress*disp).g,
                getFromColor(p + progress*disp*(1.0 + colorSeparation)).b,
                1.0);
              return texTo*progress + texFrom*inv;
            }
    "#,
    r#"
            const float amplitude = 30.0;
            const float speed = 30.0;

            vec4 transition(vec2 p) {
              vec2 dir = p - vec2(.5);
              float dist = length(dir);
              vec2 offset = dir * sin(dist * amplitude - progress * speed);
              return mix(getFromColor(p + offset), getToColor(p), progress);
            }
    "#,
];

pub fn get_trans_fragment_src() -> Vec<String> {
    let mut tfs = vec![];
    for t in TRANS_FS {
        tfs.push(format!(
            r#"
            precision highp float;
            out vec4 FragColor;
            in vec2 TexCoord;
            uniform sampler2D texture1;
            uniform sampler2D texture2;
            uniform float progress;
            vec4 getFromColor(vec2 uv) {{ return texture(texture1, uv); }}
            vec4 getToColor(vec2 uv) {{ return texture(texture2, uv); }}
            {}
            void main() {{ FragColor =  transition(TexCoord); }}
            "#,
            t
        ));
    }
    tfs
}

/// General2D 顶点着色器
///
/// # 输入顶点数据 (全屏四边形)
/// ```text
/// aPos (顶点位置, NDC坐标):        aTexCoord (纹理坐标):
///   (-1, 1)───(1, 1)                 (0, 1)───(1, 1)
///      │         │                      │         │
///      │         │         ───►         │         │
///      │         │                      │         │
///   (-1,-1)───(1,-1)                 (0, 0)───(1, 0)
/// ```
///
/// # Uniform 参数
/// - **transform**: 4x4 变换矩阵，控制顶点位置（缩放+平移）
/// - **area**: [x, y, w, h] 纹理采样区域，UV 坐标 (0.0-1.0)
///
/// # 纹理坐标计算
/// ```text
/// TexCoord.x = mix(area.x, area.x + area.z, aTexCoord.x)
///            = area.x + area.z * aTexCoord.x
///            = 采样起始X + 采样宽度 * 插值
///
/// TexCoord.y = mix(area.y, area.y + area.w, aTexCoord.y)
///            = area.y + area.w * aTexCoord.y
///            = 采样起始Y + 采样高度 * 插值
/// ```
pub const GENERAL2D_VERTEX_SRC: &str = r#"
            precision highp float;

            // 顶点属性
            layout(location = 0) in vec2 aPos;       // 顶点位置 (NDC: -1 到 1)
            layout(location = 1) in vec2 aTexCoord;  // 纹理坐标 (UV: 0 到 1)

            // 传递给片段着色器
            out vec2 TexCoord;

            // Uniform 参数
            uniform mat4 transform;  // 变换矩阵 (scale + translate)
            uniform vec4 area;       // 纹理采样区域 [x, y, width, height]

            void main()
            {
                // 计算实际纹理坐标
                // mix(a, b, t) = a + (b - a) * t = a * (1-t) + b * t
                // 当 aTexCoord = (0,0) 时, TexCoord = (area.x, area.y)
                // 当 aTexCoord = (1,1) 时, TexCoord = (area.x+area.z, area.y+area.w)
                TexCoord = vec2(
                    mix(area.x, area.x + area.z, aTexCoord.x),
                    mix(area.y, area.y + area.w, aTexCoord.y)
                );

                // 应用变换矩阵计算最终顶点位置
                // transform 包含: scale(vp_w/pcw, vp_h/pch) + translate(tx, ty)
                gl_Position = transform * vec4(aPos, 0.0, 1.0);
            }
        "#;

/// General2D 片段着色器
///
/// # 功能
/// 从纹理采样颜色，乘以 color uniform（用于 alpha 混合）
///
/// # 参数
/// - **texture1**: RT 纹理
/// - **color**: 颜色乘数 (通常是 [1, 1, 1, alpha])
pub const GENERAL2D_FRAGMENT_SRC: &str = r#"
            precision highp float;

            out vec4 FragColor;           // 输出颜色
            in vec2 TexCoord;             // 从顶点着色器传入的纹理坐标

            uniform sampler2D texture1;   // RT 纹理
            uniform vec4 color;           // 颜色乘数 (用于 alpha)

            void main()
            {
                // 采样纹理
                vec4 texColor = texture(texture1, TexCoord);

                // 乘以颜色 (应用 alpha 透明度)
                // color = (1.0, 1.0, 1.0, alpha)
                // 所以 FragColor.a = texColor.a * alpha
                FragColor = texColor * color;
            }
        "#;
