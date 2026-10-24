import type { Snapshot } from "../api/types";
import { defaultPlacement } from "../house/layout";
import type { Point3, Resident } from "../house/life";

export const W=640, H=520;
export function project(x:number,y:number,z=0):[number,number] {return [Math.round(300+(x-y)*20),Math.round(205+(x+y)*10-z*20)];}
export function unproject(px:number,py:number,z=0):Point3 {const a=(px-300)/20,b=(py-205+z*20)/10;return {x:(a+b)/2,y:(b-a)/2,z};}
const C={ink:"#292735",wood:"#a76552",edge:"#663f43",light:"#e8ad77",cream:"#f1d6a2",leaf:"#587e6b",leaf2:"#88a779",pink:"#dca0a1",blue:"#758eaf"};
type Context=CanvasRenderingContext2D;
function rect(c:Context,x:number,y:number,w:number,h:number,color:string) {c.fillStyle=color;c.fillRect(Math.round(x),Math.round(y),Math.round(w),Math.round(h));}
function poly(c:Context,points:[number,number][],color:string) {c.fillStyle=color;c.beginPath();points.forEach(([x,y],i)=>i?c.lineTo(Math.round(x),Math.round(y)):c.moveTo(Math.round(x),Math.round(y)));c.closePath();c.fill();}
function line(c:Context,points:[number,number][],color:string,width=1) {c.strokeStyle=color;c.lineWidth=width;c.beginPath();points.forEach(([x,y],i)=>i?c.lineTo(Math.round(x)+.5,Math.round(y)+.5):c.moveTo(Math.round(x)+.5,Math.round(y)+.5));c.stroke();}
function ellipse(c:Context,x:number,y:number,rx:number,ry:number,color:string) { // Stepped, deliberately pixel-edged ovals.
  for(let yy=-Math.ceil(ry);yy<=ry;yy++) {const span=Math.floor(rx*Math.sqrt(Math.max(0,1-yy*yy/(ry*ry))));if(span)rect(c,x-span,y+yy,span*2,1,color);}
}
function tile(c:Context,x:number,y:number,z:number,color:string,sx=1,sy=1) {poly(c,[project(x,y,z),project(x+sx,y,z),project(x+sx,y+sy,z),project(x,y+sy,z)],color);}
function box(c:Context,x:number,y:number,z:number,sx:number,sy:number,height:number,top:string,left:string,right:string) {
  poly(c,[project(x,y+sy,z),project(x+sx,y+sy,z),project(x+sx,y+sy,z+height),project(x,y+sy,z+height)],left);
  poly(c,[project(x+sx,y,z),project(x+sx,y+sy,z),project(x+sx,y+sy,z+height),project(x+sx,y,z+height)],right);
  tile(c,x,y,z+height,top,sx,sy);
}
function pot(c:Context,x:number,y:number,z:number,flower=false) {
  const [px,py]=project(x,y,z);ellipse(c,px,py,6,3,"#3d334438");
  poly(c,[[px-5,py-8],[px+5,py-8],[px+3,py],[px-3,py]],"#b97061");rect(c,px-6,py-9,12,3,"#e29d7b");
  rect(c,px-1,py-22,2,14,"#698960");
  for(const [a,b] of [[-5,-15],[4,-18],[-3,-22],[2,-26]]){ellipse(c,px+a,py+b,4,2,C.leaf2);if(flower){rect(c,px+a-2,py+b-3,5,4,C.pink);rect(c,px+a,py+b-2,1,1,"#ffe4a1");}}
}
function candle(c:Context,x:number,y:number,z:number,t:number) {const [px,py]=project(x,y,z);rect(c,px-2,py-10,4,10,"#f3d5a6");rect(c,px-1,py-15+(Math.floor(t*3)%2),2,5,"#ffcf7f");rect(c,px,py-13,1,2,"#fff2bf");}
function windowPane(c:Context,x:number,night:boolean) {
  const a=project(x,.02,4.8),b=project(x+2,.02,6.7);
  poly(c,[[a[0]-3,a[1]+2],[b[0]+3,a[1]+22],[b[0]+3,b[1]-3],[a[0]-3,b[1]-23]],C.edge);
  const points=[project(x,.04,4.9),project(x+2,.04,4.9),project(x+2,.04,6.6),project(x,.04,6.6)];
  poly(c,points,night?"#b3b0cb":"#9ed1c5");
  line(c,[project(x+1,.05,4.9),project(x+1,.05,6.6)],C.cream,2);
  line(c,[project(x,.05,5.7),project(x+2,.05,5.7)],C.cream,2);
  box(c,x-.15,.05,4.8,2.3,.35,.12,"#e0b58c",C.wood,C.edge);
}
export function drawSoot(c:Context,px:number,py:number,t:number,moving=false,sleeping=false,facing=1,action="") {
  px=Math.round(px);py=Math.round(py);
  ellipse(c,px,py+1,10,3,"#26202e40");
  const step=moving?Math.sin(t*15):0;
  // Feet stay on their tile. Only the body has a one-pixel walking bob.
  if(!sleeping){rect(c,px-6,py-2+Math.max(0,Math.round(step*2)),4,2,C.ink);rect(c,px+3,py-2+Math.max(0,Math.round(-step*2)),4,2,C.ink);}
  const cy=py-10-(moving?Math.round(Math.abs(step)):0);
  for(let i=0;i<28;i++){const a=i*Math.PI/14;const r=i%2?10:11;rect(c,px+Math.cos(a)*r,cy+Math.sin(a)*r,1,2,"#252431");}
  ellipse(c,px,cy,10,sleeping?8:10,"#252431");rect(c,px-7,cy-7,10,2,"#33313c");
  if(sleeping) {line(c,[[px-7,cy],[px-5,cy+1],[px-3,cy]],"#e9e2d7");line(c,[[px+2,cy],[px+4,cy+1],[px+6,cy]],"#e9e2d7");}
  else {rect(c,px-7,cy-4,6,7,"#fff5d9");rect(c,px+1,cy-4,6,7,"#fff5d9");rect(c,px-5+facing,cy-2,2,3,"#23212c");rect(c,px+3+facing,cy-2,2,3,"#23212c");}
  const arm=moving?Math.round(step*2):0;rect(c,px-13,cy+4+arm,5,1,C.ink);rect(c,px+9,cy+4-arm,5,1,C.ink);
  if(sleeping){const y=cy-17-Math.floor(t%3);c.fillStyle="#e8d5ad";c.font="7px monospace";c.fillText("z",px+12,y);}
  else if(!moving&&action==="tea"){rect(c,px+9,cy+1,6,5,"#eac1a0");rect(c,px+15,cy+2,2,3,"#eac1a0");rect(c,px+11,cy-3-Math.floor(t%2),1,3,"#e3d6b3");}
  else if(!moving&&["books","desk"].includes(action)){rect(c,px-8,cy+4,17,7,"#acb69c");rect(c,px,cy+4,1,7,"#ece4bb");}
  else if(!moving&&action==="hall"){line(c,[[px+10,cy-9],[px+16,py+1]],"#aa7857",2);rect(c,px+12,py-1,9,3,"#d6b27b");}
}
function companion(c:Context,asset:string,x:number,y:number,t:number) {
  if(/cat|jiji/.test(asset)) {
    ellipse(c,x,y-6,5,7,"#2d2a39");ellipse(c,x,y-17,7,6,"#2d2a39");poly(c,[[x-7,y-19],[x-6,y-30],[x-1,y-22]],"#2d2a39");poly(c,[[x+1,y-22],[x+7,y-30],[x+7,y-18]],"#2d2a39");rect(c,x-5,y-19,3,4,"#ffe7bb");rect(c,x+2,y-19,3,4,"#ffe7bb");rect(c,x-4,y-18,1,2,C.ink);rect(c,x+3,y-18,1,2,C.ink);line(c,[[x+3,y-2],[x+10,y-4],[x+12,y-14],[x+10,y-19]],"#2d2a39",3);
  } else if(/dragon|haku/.test(asset)) {
    const points:[[number,number],[number,number],[number,number],[number,number],[number,number],[number,number]]=[[x-15,y-3],[x-6,y-8],[x+8,y-5],[x+16,y-11],[x+8,y-21],[x-3,y-21]];
    line(c,points,"#466f75",8);line(c,points,"#d9e4d1",5);rect(c,x-12,y-26,13,7,"#edf0db");poly(c,[[x-10,y-26],[x-4,y-33],[x-4,y-25]],"#81ada4");rect(c,x-11,y-25,2,2,C.ink);line(c,[[x-10,y-20],[x-18,y-23],[x-23,y-20]],"#cbd9c4");
  } else if(/dog|totoro/.test(asset)) {
    ellipse(c,x,y-10,10,13,"#628b9e");poly(c,[[x-8,y-18],[x-7,y-32],[x-2,y-22]],"#628b9e");poly(c,[[x+2,y-22],[x+8,y-32],[x+8,y-18]],"#628b9e");ellipse(c,x,y-8,7,8,"#f4dfb6");rect(c,x-6,y-21,4,4,"#fff3d1");rect(c,x+3,y-21,4,4,"#fff3d1");rect(c,x-4,y-20,1,2,C.ink);rect(c,x+4,y-20,1,2,C.ink);ellipse(c,x+13,y-5,4,6,"#f5e7c9");rect(c,x+10,y-15,2,7,"#f5e7c9");rect(c,x+14,y-14,2,7,"#f5e7c9");
  } else if(/ootori/.test(asset)) {
    ellipse(c,x,y-9,10,12,"#edcb75");rect(c,x-7,y-24,12,4,"#f9e5bd");rect(c,x-6,y-17,4,4,"#fff7db");rect(c,x+3,y-17,4,4,"#fff7db");rect(c,x-4,y-16,1,2,C.ink);rect(c,x+4,y-16,1,2,C.ink);poly(c,[[x-3,y-11],[x+2,y-14],[x+7,y-10]],"#d78f58");
  } else if(/aogaeru/.test(asset)) {
    ellipse(c,x,y-7,11,8,"#8aacc1");ellipse(c,x,y-18,11,7,"#78a66d");rect(c,x-8,y-25,5,5,"#eef0bb");rect(c,x+4,y-25,5,5,"#eef0bb");rect(c,x-6,y-24,2,3,C.ink);rect(c,x+6,y-24,2,3,C.ink);line(c,[[x-5,y-15],[x+5,y-15]],"#44604c");
  } else {
    ellipse(c,x,y-9,10,12,"#baaaa5");ellipse(c,x-7,y-22,4,5,"#c99e99");ellipse(c,x+7,y-22,4,5,"#c99e99");ellipse(c,x,y-10,7,8,"#e2d2ad");rect(c,x-6,y-19,4,4,"#ece3d3");rect(c,x+3,y-19,4,4,"#ece3d3");rect(c,x-4,y-18,1,2,C.ink);rect(c,x+4,y-18,1,2,C.ink);ellipse(c,x,y-30,4,4,"#302b36");rect(c,x-2,y-31,2,2,"#eee4c6");rect(c,x+2,y-31,2,2,"#eee4c6");rect(c,x-1,y-27,5,2,"#d5b56b");
  }
  if(t<0) return;
}
export function drawItem(c:Context,asset:string,x:number,y:number,z=0,t=0) {
  const [px,py]=project(x,y,z);
  if(asset.startsWith("pets/")){companion(c,asset,px,py,t);return;}
  if(/plant/.test(asset)){pot(c,x,y,z);return;}
  if(/bed/.test(asset)) {
    box(c,x-.7,y-1,z,1.5,2.3,.4,"#c2a19d",C.wood,C.edge);box(c,x-.8,y-1.1,z,1.7,.2,1.2,"#e1b585",C.wood,C.edge);
    box(c,x-.62,y-.8,z+.4,1.3,.6,.15,"#f1dcb4","#d4bea0","#d4bea0");box(c,x-.68,y-.1,z+.4,1.4,1.3,.2,"#8ba6a1","#657f80","#657f80");
    for(let i=0;i<4;i++)line(c,[project(x-.6,y+i*.3,z+.61),project(x+.6,y+i*.3,z+.61)],"#b8c6aa");return;
  }
  if(/couch/.test(asset)){box(c,x-1,y-.5,z,2,1,.5,"#c49798","#a27484","#8b6177");box(c,x-1,y-.65,z+.3,2,.3,.7,"#e2b2a5","#b58290","#8b6177");box(c,x-.95,y+.1,z+.5,.8,.4,.2,"#edd0a6","#d6ab94","#b58290");return;}
  if(/chair/.test(asset)) {
    for(const dx of [-.32,.32])for(const dy of [-.28,.28])box(c,x+dx,y+dy,z,.1,.1,.5,"#c4936d",C.wood,C.edge);
    box(c,x-.4,y-.35,z+.5,.8,.7,.13,"#a7b29a","#829582","#667e73");
    box(c,x-.4,y+.28,z+.6,.8,.12,.65,"#d6ab7f","#b17e63","#8c6156");return;
  }
  if(/desk|tea-table/.test(asset)) {
    const big=/desk/.test(asset);const a=big?1.1:.8;
    for(const dx of [-a+.1,a-.1])for(const dy of [-.3,.3])box(c,x+dx,y+dy,z,.12,.12,.7,"#ad795a",C.wood,C.edge);
    box(c,x-a,y-.5,z+.7,a*2,1,.15,"#e0b07a",C.wood,C.edge);
    if(big){rect(c,px-5,py-21,15,3,"#e2d9b2");rect(c,px+4,py-28,5,7,"#a6b7a0");}else{rect(c,px-2,py-23,6,5,"#e3c9a0");rect(c,px+4,py-22,2,3,"#e3c9a0");}return;
  }
  if(/bookshelf/.test(asset)) {
    box(c,x-1,y-.2,z,2,.5,2,"#d6a574",C.wood,C.edge);
    for(let shelf=0;shelf<3;shelf++)for(let i=0;i<7;i++)box(c,x-.9+i*.25,y+.32,z+.12+shelf*.57,.17,.15,.3+(i%3)*.06,["#d7b184","#879c8c","#aa929e","#7d8da4"][i%4],"#5c5964","#5c5964");return;
  }
  if(/poster|painting|window/.test(asset)){rect(c,px-17,py-37,34,26,C.edge);rect(c,px-15,py-35,30,22,"#d9b17e");rect(c,px-12,py-32,24,16,"#8da69d");if(/whale/.test(asset)){c.save();c.translate(px-2,py-13);c.scale(.55,.55);companion(c,"pets/haku",0,0,0);c.restore();}else drawSoot(c,px,py-13,0,false,false);return;}
  if(/mushroom|lamp/.test(asset)){rect(c,px-1,py-27,3,26,"#bd926c");ellipse(c,px,py,7,3,C.edge);if(/mushroom/.test(asset)){ellipse(c,px,py-25,12,8,"#ce8d81");rect(c,px-12,py-25,24,4,"#e3bc95");rect(c,px-6,py-29,3,2,"#f3dab2");rect(c,px+3,py-31,4,3,"#f3dab2");}else{poly(c,[[px-5,py-35],[px+5,py-35],[px+11,py-23],[px-11,py-23]],"#ecd1a0");}return;}
  if(/rug/.test(asset)){tile(c,x-1.4,y-.8,z+.02,"#a87a8e",2.8,1.6);tile(c,x-1.1,y-.6,z+.03,"#d4a49f",2.2,1.2);tile(c,x-.8,y-.4,z+.04,"#a87a8e",1.6,.8);return;}
  if(/cushion/.test(asset)){ellipse(c,px,py-3,14,6,"#e1c6bc");ellipse(c,px-7,py-7,6,5,"#eadbc1");ellipse(c,px+4,py-9,8,6,"#eadbc1");return;}
  if(/pc/.test(asset)){rect(c,px-11,py-25,24,17,C.edge);rect(c,px-9,py-23,20,13,"#7caaa0");rect(c,px-1,py-9,4,7,C.edge);rect(c,px-8,py-2,18,2,"#ae9f97");rect(c,px-6,py-21,2,2,"#d4e4b7");return;}
  box(c,x-.5,y-.4,z,1,.8,.6,"#b5b9a1","#7d9a8a","#58776d");
}
function floor(c:Context,z:number,maxY:number,upper=false) {
  box(c,0,0,z-.3,12,maxY,.3,"#dba479","#966253","#67474a");
  for(let x=0;x<12;x++)for(let y=0;y<maxY;y++){
    tile(c,x+.025,y+.025,z+.01,["#c39173","#d0a17d","#c99a7a","#d7ac85"][(x*7+y*3)%4],.95,.95);
    if((x*3+y)%5===0)line(c,[project(x+.15,y+.4,z+.02),project(x+.7,y+.4,z+.02)],"#bb886d");
    const p=project(x+.2,y+.6,z+.02);rect(c,p[0],p[1],3,1,"#e4b58a");rect(c,p[0]+4,p[1]+3,2,1,"#b9856c");
  }
  if(upper)line(c,[project(0,maxY,z),project(9.8,maxY,z)],"#f0c58d",2);
}
function walls(c:Context,s:Snapshot,upper:boolean) {
  const z=upper?4:0, height=upper?3.3:4;
  const paper=upper?s.home?.wallpapers.loft:s.home?.wallpapers.hearth;
  const colors=paper?.includes("fern")?["#9cb7a0","#7b9b8a"]:paper?.includes("starlight")?["#9595b5","#777c9d"]:paper?.includes("rose")?["#d4aaa4","#b88f92"]:["#b6bba0","#8eaa99"];
  poly(c,[project(0,0,z),project(12,0,z),project(12,0,z+height),project(0,0,z+height)],colors[0]);
  if(!upper){
    const studio=s.home?.wallpapers.studio;
    const ink=studio?.includes("fern")?"#9cb7a0":studio?.includes("starlight")?"#9595b5":studio?.includes("rose")?"#d4aaa4":"#b6bba0";
    poly(c,[project(6,0,0),project(12,0,0),project(12,0,4),project(6,0,4)],ink);
  }
  poly(c,[project(0,0,z),project(0,upper?4:9,z),project(0,upper?4:9,z+height),project(0,0,z+height)],colors[1]);
  for(let y=0;y<(upper?4:9);y+=.5)line(c,[project(0,y,z+.1),project(0,y,z+height)],"#546d6530");
  for(let x=0;x<=12;x+=3)box(c,x,.03,z,.15,.12,height,"#b98663","#97664f","#75504a");
  line(c,[project(0,0,z+height),project(12,0,z+height)],"#e8b785",4);
  line(c,[project(0,0,z+height),project(0,upper?4:9,z+height)],"#b47f65",4);
  if(paper?.includes("starlight"))for(let x=1;x<12;x++)for(let zz=z+.5;zz<z+height;zz+=.7){const [px,py]=project(x,.01,zz);rect(c,px,py,2,2,"#d9cdab");}
}
function stairs(c:Context) {
  for(let i=0;i<12;i++) {
    const y=8.3-i*.4,z=i/3;
    box(c,10,y-.4,z,1.5,.4,1/3,"#e9b582","#ad7b63","#745150");
    line(c,[project(10,y,z+1/3),project(11.5,y,z+1/3)],"#f2c998");
  }
}
function foliage(c:Context,x:number,y:number,scale=1,flowers=false) {
  for(let i=0;i<6;i++) {const px=x+Math.sin(i*4)*12*scale,py=y+Math.cos(i*3)*5*scale;
    ellipse(c,px,py,9*scale,8*scale,i%2?"#4c7468":"#648975");
    rect(c,px-3,py-6,6,2,"#88a083");if(flowers&&i%2===0){rect(c,px,py-3,3,3,"#d8a2a0");rect(c,px+1,py-2,1,1,"#f9ddb0");}
  }
}
function background(c:Context,t:number,night:boolean) {
  rect(c,0,0,W,H,night?"#292b42":"#7c9e9d");
  for(let i=0;i<6;i++) {
    const y=60+i*62;
    poly(c,[[0,y+70],[50,y+35],[120,y+67],[208,y+10],[295,y+78],[390,y+37],[470,y+71],[565,y+15],[640,y+60],[640,H],[0,H]],night?["#30334b","#343b50","#3c4758","#41585d","#466363","#496864"][i]:["#88a6a3","#839f9b","#75958b","#6a8d7c","#628572","#587e6b"][i]);
  }
  for(let i=0;i<45;i++){const x=(i*139+17)%640,y=(i*43)%190;const on=(Math.floor(t/2)+i)%5!==0;if(on){rect(c,x,y,i%4===0?2:1,1,night?"#c5b7b1":"#c9d5bc");if(i%7===0)rect(c,x,y-1,1,3,night?"#c5b7b1":"#c9d5bc");}}
  ellipse(c,551,91,19,19,night?"#eddec2":"#e6d6ab");ellipse(c,560,86,18,18,night?"#343b50":"#88a6a3");
  for(const [x,y,s] of [[43,322,1.6],[591,354,1.8],[62,423,1.3],[560,441,1.5],[24,479,1.9],[618,495,1.7]])foliage(c,x,y,s,true);
}
export function furniturePosition(asset:string,placement:{room:string;x:number;y:number}):Point3 {
  const wall=/poster|painting|window/.test(asset);
  if(placement.room==="loft")return {x:1+(placement.x-30)/260*8,y:wall ? .12 : .8+(placement.y-155)/60*2.4,z:wall?4.7:4};
  // Ground-floor pictures hang from the mezzanine beam, clear of walk paths.
  return {x:(placement.room==="studio"?6.2:1)+(placement.x-30)/260*(placement.room==="studio"?3:4),y:wall?4.05:5.1+(placement.y-155)/60*2.4,z:wall?2.1:0};
}
const STARTER_SPOTS:Record<string,Point3>={
  "furniture/couch":{x:3.3,y:5.25,z:0},"furniture/tea-table":{x:3.3,y:7,z:0},
  "furniture/desk":{x:7.4,y:4.7,z:0},"furniture/rug":{x:3.3,y:6.8,z:0},
  "furniture/bed":{x:2.2,y:1.1,z:4},"furniture/cloud-cushion":{x:8.1,y:2.35,z:4.05},
};
export function drawHouse(c:Context,s:Snapshot,r:Resident,t:number,night:boolean,editing=false) {
  background(c,t,night);
  box(c,-.8,-.6,-1.1,13.6,10.3,.5,"#799271","#4e6d60","#3b5657");
  for(let i=0;i<30;i++){const p=project(-.7+(i*23%134)/10,-.5+(i*47%102)/10,-.59);rect(c,p[0],p[1],2,2,i%2?"#b1b38a":"#6b8d70");}
  walls(c,s,false);floor(c,0,9);
  // Fireplace, kitchen shelf and small amenities are part of the starter home.
  box(c,.15,4.5,0,.9,1.8,1.65,"#c5b29b","#9c9383","#7b7773");
  const [fx,fy]=project(.9,5.5,.3);rect(c,fx-9,fy-22,19,23,"#514754");rect(c,fx-8,fy-2,18,3,"#ae7761");
  for(let i=0;i<4;i++)rect(c,fx-7+i*4,fy-8-Math.floor(Math.sin(t*5+i)*3),3,8,"#efb779");
  box(c,.1,7.3,0,.8,1.1,1,"#c8a077","#a77e65","#7b5c56");pot(c,.5,7.8,1,true);
  // Little shelves, copper pots and climbing leaves on the cutaway wall.
  box(c,.05,6.1,2.2,.65,2,.12,"#deb184","#ac7d63","#805b53");
  pot(c,.35,6.4,2.32);pot(c,.35,7.4,2.32,true);
  for(let i=0;i<4;i++){const p=project(.1,4.4+i*.22,2.9);line(c,[[p[0],p[1]],[p[0],p[1]+7]],"#bb9c76");ellipse(c,p[0],p[1]+10,3,4,i%2?"#b67961":"#d4a678");}
  // The sofa faces its coffee table. The study sits back from the stairwell.
  const placed=s.inventory.filter(o=>o.equipped&&!["outfit","wallpaper","room"].includes(o.item.category)).map(o=>{
    const anchor=defaultPlacement(o.item),placement=s.home?.placements[o.item.id]??anchor;
    const slot=STARTER_SPOTS[o.item.asset_path];
    const unchanged=placement.room===anchor.room&&placement.x===anchor.x&&placement.y===anchor.y;
    return {...o,p:slot&&unchanged?slot:furniturePosition(o.item.asset_path,placement)};
  });
  const collected=(asset:string,z:number)=>placed.some(o=>o.item.asset_path===asset&&Math.abs(o.p.z-z)<.1);
  if(!collected("furniture/rug",0))drawItem(c,"furniture/rug",3.3,6.8,0);
  for(const o of placed.filter(o=>o.item.asset_path==="furniture/rug"&&o.p.z<4))drawItem(c,o.item.asset_path,o.p.x,o.p.y,o.p.z,t);
  const desk=placed.find(o=>o.item.asset_path==="furniture/desk"&&o.p.z===0)?.p??STARTER_SPOTS["furniture/desk"];
  const teaTable=placed.find(o=>o.item.asset_path==="furniture/tea-table"&&o.p.z===0)?.p??STARTER_SPOTS["furniture/tea-table"];
  const starter=[
    {asset:"furniture/couch",x:3.3,y:5.25},
    {asset:"furniture/tea-table",x:3.3,y:7},
    {asset:"furniture/desk",x:7.4,y:4.7},
    {asset:"furniture/chair",x:desk.x,y:desk.y+1.4},
  ].filter(o=>!collected(o.asset,0));
  const downstairs=[...starter.map(o=>({asset:o.asset,p:{x:o.x,y:o.y,z:0}})),...placed.filter(o=>o.p.z<4&&o.item.asset_path!=="furniture/rug").map(o=>({asset:o.item.asset_path,p:o.p}))].sort((a,b)=>(a.p.x+a.p.y)-(b.p.x+b.p.y));
  const drawResident=()=>{const asleep=!r.moving&&r.node==="bed",seated=!r.moving&&r.node==="desk";const [x,y]=project(r.position.x,r.position.y,r.position.z+((asleep||seated) ? .5 : 0));drawSoot(c,x,y,t,r.moving,asleep,r.facing,r.node);};
  for(const o of downstairs)drawItem(c,o.asset,o.p.x,o.p.y,o.p.z,t);
  candle(c,teaTable.x-.15,teaTable.y-.05,.88,t);pot(c,desk.x+.65,desk.y-.05,.85);
  if(r.position.z===0)drawResident();
  // The mezzanine is a single part of the same house, not another room view.
  walls(c,s,true);floor(c,4,4,true);
  windowPane(c,2,night);windowPane(c,7,night);
  // Headboard against the wall; books between windows; a reading nook to the right.
  if(!collected("furniture/bed",4))drawItem(c,"furniture/bed",2.2,1.1,4);
  drawItem(c,"furniture/bookshelf",5.5,.25,4);
  box(c,3.5,.2,4,.7,.75,.65,"#e0b489","#b58569","#956c60");candle(c,3.85,.6,4.67,t);
  tile(c,7,1.7,4.03,"#9a879f",2.1,1.2);tile(c,7.25,1.9,4.04,"#d1aaa8",1.6,.8);
  if(!collected("furniture/cloud-cushion",4.05))drawItem(c,"furniture/cloud-cushion",8.1,2.35,4.05);
  pot(c,11,.5,4,true);
  // Curtains and climbing greenery are drawn with the same pixel palette.
  for(const xx of [1.65,4.2,6.65,9.2]){
    const a=project(xx,.1,6.65),b=project(xx,.1,4.95);
    poly(c,[[a[0]-4,a[1]],[a[0]+3,a[1]+3],[b[0]+2,b[1]-2],[b[0]-5,b[1]-5]],xx<5?"#cba1a2":"#aab79a");
    rect(c,b[0]-4,b[1]-11,6,2,"#dfbd93");
  }
  pot(c,.5,2.7,4,true);
  for(const o of placed.filter(o=>o.p.z>=4).sort((a,b)=>(a.p.x+a.p.y)-(b.p.x+b.p.y)))drawItem(c,o.item.asset_path,o.p.x,o.p.y,o.p.z,t);
  stairs(c);
  if(r.position.z>0)drawResident();
  // Open banisters frame the walkable balcony and staircase.
  for(let x=0;x<9.7;x+=.8)line(c,[project(x,4,4),project(x,4,4.65)],"#a06b58",2);
  line(c,[project(0,4,4.65),project(9.7,4,4.65)],"#f0bd89",3);
  for(let i=0;i<=6;i++)line(c,[project(11.55,8.3-i*.8,i*2/3),project(11.55,8.3-i*.8,i*2/3+.75)],"#8d5f56",2);
  line(c,[project(11.55,8.3,.8),project(11.55,3.5,4.8)],"#f1c18e",3);
  // Timber roof edges give the cutaway its house silhouette without hiding play.
  line(c,[project(-.15,-.15,7.55),project(12.2,-.15,7.55)],"#6b4850",9);
  line(c,[project(-.15,-.15,7.7),project(12.2,-.15,7.7)],"#bc8075",5);
  line(c,[project(-.15,-.15,7.55),project(-.15,4.2,7.55)],"#6b4850",9);
  line(c,[project(-.15,-.15,7.7),project(-.15,4.2,7.7)],"#bc8075",5);
  // String lights, flowers and chimney smoke keep the house alive.
  line(c,[project(1,4.05,5.2),project(5,4.05,4.9),project(9,4.05,5.2)],"#64605b");
  for(let x=1;x<9;x++){const p=project(x,4.05,5.1-Math.sin(x/9*Math.PI)*.25);rect(c,p[0],p[1],2,3,x%2?"#f5cd8d":"#d7aba2");}
  if(editing)for(let x=1;x<10;x++)for(let y=5;y<9;y++)line(c,[project(x,y),project(x+1,y),project(x+1,y+1),project(x,y+1),project(x,y)],"#f5e3bc55");
  for(let i=0;i<9;i++){const x=70+(i*59)%510+Math.sin(t*.35+i)*6,y=320+(i*37)%145+Math.cos(t*.4+i)*4;rect(c,x,y,2,2,Math.sin(t+i)>.1?"#d6d5a7":"#a6be8b");}
}

export const FLOWERS=[{id:"daisy",name:"Starlight daisy",color:"#f1dfb8"},{id:"poppy",name:"Coral poppy",color:"#de9191"},{id:"moonflower",name:"Moonflower",color:"#b9acd4"}];
export const GROW_SECONDS=1800;
export function flowerStage(flower:{planted_at:number;watered_at:number|null},now:number) {return flower.watered_at===null?0:Math.min(3,Math.floor(Math.max(0,now-flower.planted_at)/GROW_SECONDS*3));}
export const plotPoint=(i:number)=>project(3+(i%3)*2.4,3+Math.floor(i/3)*2.7,0);
export function drawGarden(c:Context,s:Snapshot,t:number,night:boolean,now:number) {
  background(c,t,night);
  box(c,0,0,-.6,12,10,.6,"#8a9b77","#5d7a65","#415e58");
  // Stone path and a gate back to the house.
  for(let y=0;y<10;y++)tile(c,9,y,.02,y%2?"#bab599":"#c7b99d",1.2,.9);
  for(let x=0;x<12;x+=.55)box(c,x,0,0,.2,.15,1.4,"#d6ba8e","#a78770","#8c6c62");
  line(c,[project(0,0,.55),project(12,0,.55)],"#c1a27e",3);line(c,[project(0,0,1.1),project(12,0,1.1)],"#ddc299",3);
  for(let y=.5;y<10;y+=.6)box(c,0,y,0,.15,.18,1.4,"#d6ba8e","#a78770","#8c6c62");
  for(let i=0;i<6;i++) {
    const x=3+i%3*2.4,y=3+Math.floor(i/3)*2.7;
    box(c,x-.85,y-.8,0,1.7,1.6,.2,"#7b5c56","#ad8166","#70514e");
    for(let j=0;j<5;j++)line(c,[project(x-.6,y-.6+j*.28,.21),project(x+.6,y-.6+j*.28,.21)],"#916c59");
    const f=s.home?.garden?.[i];
    if(f){const stage=flowerStage(f,now);const color=FLOWERS.find(k=>k.id===f.species)?.color??"#f1dfb8";
      for(const [dx,dy] of [[-.35,-.25],[.3,.2],[-.25,.4]]){
        const [px,py]=project(x+dx,y+dy,.25);const height=3+stage*5;
        rect(c,px,py-height,1,height,"#84a276");rect(c,px-3,py-height/2,3,2,"#9eb683");rect(c,px+1,py-height/2-3,3,2,"#9eb683");
        if(stage>=2){ellipse(c,px,py-height,stage===3?5:3,stage===3?4:2,color);rect(c,px-1,py-height-1,2,2,"#e6ba79");}
      }
      if(f.watered_at===null){const p=project(x,y,.3);rect(c,p[0]+8,p[1]-17,3,5,"#a4c3ce");}
    }
  }
  const [px,py]=project(9.4,7.2);drawSoot(c,px,py,t,false,false,1,"tea");
  for(const [x,y] of [[1,1],[11,1],[1,9],[11,9]]){const p=project(x,y);foliage(c,p[0],p[1],.8,true);}
  box(c,1,7.8,0,2,.8,.7,"#d9b78a","#ab876b","#8e6c60");
  const p=project(10.8,3);rect(c,p[0]-6,p[1]-10,11,10,"#91b4b5");rect(c,p[0]+5,p[1]-8,7,3,"#91b4b5");line(c,[[p[0]-6,p[1]-7],[p[0]-10,p[1]-14],[p[0],p[1]-16],[p[0]+4,p[1]-10]],"#91b4b5",2);
}
