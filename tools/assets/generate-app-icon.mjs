#!/usr/bin/env node
import { deflateSync } from 'node:zlib';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
function crc32(b){let c=0xffffffff;for(const x of b){c^=x;for(let i=0;i<8;i++)c=(c>>>1)^(0xedb88320&-(c&1));}return(c^0xffffffff)>>>0;}
function chunk(t,d){const n=Buffer.from(t),o=Buffer.alloc(12+d.length);o.writeUInt32BE(d.length,0);n.copy(o,4);d.copy(o,8);o.writeUInt32BE(crc32(Buffer.concat([n,d])),8+d.length);return o;}
const w=256,h=256,p=Buffer.alloc(w*h*4);for(let y=0;y<h;y++)for(let x=0;x<w;x++){const i=(y*w+x)*4,dx=x-128,dy=y-128,inside=dx*dx+dy*dy<116*116;p[i]=inside?Math.round(105+55*x/w):0;p[i+1]=inside?Math.round(79+45*y/h):0;p[i+2]=inside?205:0;p[i+3]=inside?255:0;}
function rect(x,y,rw,rh,c){for(let py=y;py<y+rh;py++)for(let px=x;px<x+rw;px++){const i=(py*w+px)*4;p[i]=c[0];p[i+1]=c[1];p[i+2]=c[2];p[i+3]=255;}}
const white=[241,238,249];rect(65,65,24,126,white);rect(65,65,75,22,white);rect(65,112,62,20,white);rect(153,65,24,126,white);rect(122,65,55,22,white);rect(122,112,55,20,white);rect(122,169,55,22,white);
const ih=Buffer.alloc(13);ih.writeUInt32BE(w,0);ih.writeUInt32BE(h,4);ih[8]=8;ih[9]=6;const raw=Buffer.alloc(h*(1+w*4));for(let y=0;y<h;y++)p.copy(raw,y*(1+w*4)+1,y*w*4,(y+1)*w*4);const png=Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',ih),chunk('IDAT',deflateSync(raw,{level:9})),chunk('IEND',Buffer.alloc(0))]);
const header=Buffer.alloc(22);header.writeUInt16LE(0,0);header.writeUInt16LE(1,2);header.writeUInt16LE(1,4);header[6]=0;header[7]=0;header[8]=0;header[9]=0;header.writeUInt16LE(1,10);header.writeUInt16LE(32,12);header.writeUInt32LE(png.length,14);header.writeUInt32LE(22,18);const ico=Buffer.concat([header,png]);const dir=path.resolve('apps/desktop/src-tauri/icons');await mkdir(dir,{recursive:true});await writeFile(path.join(dir,'icon.ico'),ico);await writeFile(path.join(dir,'icon.png'),png);process.stdout.write(JSON.stringify({status:'PASS',pngBytes:png.length,icoBytes:ico.length})+'\n');
