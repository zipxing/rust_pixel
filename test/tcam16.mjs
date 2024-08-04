// node tcam16.mjs
import Color from "./color.mjs";
let lime = new Color("sRGB", [0, 1, 0], 1.0);
console.log("aaa", lime.to('cam16-jmh').coords)
console.log("bbb", lime.to('hct').coords)
// console.log(lime.to('xyz').coords)
// let lime1 = new Color("cam16-jmh", [79.10134572991937, 78.2155216870714, 142.22342095435386], 1.0);
// let lxyz = lime1.to('xyz');
// console.log(lxyz.coords)
// console.log(lxyz.to('cam16-jmh').coords)
//[ 0.35758433938387796, 0.715168678767756, 0.11919477979462556 ]
