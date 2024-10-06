pub const VERTEX_SRC_SYMBOLS: &str = r#"
            precision mediump float;
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
            void main() {
                uv = a1.zw + vertex * a2.xy;
                vec2 transformed = (((vertex - a1.xy) * mat2(a2.zw, a3.xy) + a3.zw) * mat2(tw.xy, th.xy) + vec2(tw.z, th.z)) / vec2(tw.w, th.w) * 2.0;
                gl_Position = vec4(transformed - vec2(1.0, 1.0), 0.0, 1.0);
                colorj = color * colorFilter;
            }
        "#;

pub const FRAGMENT_SRC_SYMBOLS: &str = r#"
            precision mediump float;
            uniform sampler2D source;
            layout(std140) uniform transform {
                vec4 tw;
                vec4 th;
                vec4 colorFilter;
            };
            in vec2 uv;
            in vec4 colorj;
            layout(location=0) out vec4 color;
            void main() {
                color = texture(source, uv) * colorj;
            }
        "#;

// trans shader ...
pub const VERTEX_SRC_TRANS: &str = r#"
            precision mediump float;
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
            highp float noise(vec2 co)
            {
                highp float a = 12.9898;
                highp float b = 78.233;
                highp float c = 43758.5453;
                highp float dt= dot(co.xy * progress, vec2(a, b));
                highp float sn= mod(dt,3.14);
                return fract(sin(sn) * c);
            }
            vec4 transition(vec2 p) {
              if (progress < 0.05 ) {
                return getFromColor(p);
              } else if (progress > (1.0 - 0.05 )) {
                return getToColor(p);
              } else {
                return vec4(vec3(noise(p)), 1.0);
              }
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

              if (dist > progress) {
                return mix(getFromColor( p), getToColor( p), progress);
              } else {
                vec2 offset = dir * sin(dist * amplitude - progress * speed);
                return mix(getFromColor( p + offset), getToColor( p), progress);
              }
            }
    "#,
];

pub fn get_trans_fragment_src() -> Vec<String> {
    let mut tfs = vec![];
    for t in TRANS_FS {
        tfs.push(format!(
            r#"
            precision mediump float;
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

pub const GENERAL2D_VERTEX_SRC: &str = r#"
            precision mediump float;
            layout(location = 0) in vec2 aPos;
            layout(location = 1) in vec2 aTexCoord;
            out vec2 TexCoord;  
            uniform mat4 transform; 
            uniform vec4 area;      
            void main()
            {
                TexCoord = vec2(
                    mix(area.x, area.x + area.z, aTexCoord.x),
                    mix(area.y, area.y + area.w, aTexCoord.y)
                );
                gl_Position = transform * vec4(aPos, 0.0, 1.0);
            }
        "#;

pub const GENERAL2D_FRAGMENT_SRC: &str = r#"
            precision mediump float;
            out vec4 FragColor;
            in vec2 TexCoord;
            uniform sampler2D texture1;  
            uniform vec4 color;          
            void main()
            {
                vec4 texColor = texture(texture1, TexCoord);
                FragColor = texColor * color;
            }
        "#;
