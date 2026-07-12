#!/usr/bin/env node
import { deflateSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const fixtureDir = path.join(root, 'fixtures/m00/synthetic-character');
const actions = ['idle', 'run', 'jump', 'fall', 'dash', 'attack_01', 'attack_02', 'attack_03', 'hit', 'death'];

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const name = Buffer.from(type, 'ascii');
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  name.copy(out, 4);
  data.copy(out, 8);
  out.writeUInt32BE(crc32(Buffer.concat([name, data])), 8 + data.length);
  return out;
}

function encodePng(width, height, rgba) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0); header.writeUInt32BE(height, 4);
  header[8] = 8; header[9] = 6;
  const scanlines = Buffer.alloc(height * (1 + width * 4));
  for (let y = 0; y < height; y += 1) rgba.copy(scanlines, y * (1 + width * 4) + 1, y * width * 4, (y + 1) * width * 4);
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), chunk('IHDR', header), chunk('IDAT', deflateSync(scanlines, { level: 9 })), chunk('IEND', Buffer.alloc(0))]);
}

function canvas(width, height) { const pixels=Buffer.alloc(width*height*4);for(let y=0;y<height;y+=1)for(let x=0;x<width;x+=1){const i=(y*width+x)*4;pixels[i]=(x*31+y*17)&255;pixels[i+1]=(x*13+y*47)&255;pixels[i+2]=(x*7+y*11)&255;pixels[i+3]=0;}return { width, height, pixels }; }
function setPixel(c, x, y, color) {
  if (x < 0 || y < 0 || x >= c.width || y >= c.height) return;
  const i = (y * c.width + x) * 4;
  c.pixels[i] = color[0]; c.pixels[i+1] = color[1]; c.pixels[i+2] = color[2]; c.pixels[i+3] = color[3];
}
function rect(c, x, y, w, h, color) { for (let py=y; py<y+h; py+=1) for (let px=x; px<x+w; px+=1) setPixel(c, px, py, color); }
function ellipse(c, cx, cy, rx, ry, color) {
  for (let y=Math.floor(cy-ry); y<=Math.ceil(cy+ry); y+=1) for (let x=Math.floor(cx-rx); x<=Math.ceil(cx+rx); x+=1) {
    if (((x-cx)**2)/(rx**2)+((y-cy)**2)/(ry**2)<=1) setPixel(c,x,y,color);
  }
}
function line(c, x0, y0, x1, y1, thickness, color) {
  const steps = Math.max(Math.abs(x1-x0), Math.abs(y1-y0), 1);
  for (let i=0;i<=steps;i+=1) {
    const x=Math.round(x0+(x1-x0)*i/steps), y=Math.round(y0+(y1-y0)*i/steps);
    ellipse(c,x,y,thickness,thickness,color);
  }
}

const palette = { outline:[36,29,53,255], hair:[73,54,105,255], skin:[255,209,184,255], coat:[64,104,168,255], leg:[48,59,98,255], metal:[217,227,239,255], gold:[197,154,69,255] };
function drawCharacter(c, ox, oy, scale, pose=0) {
  const s=(v)=>Math.round(v*scale), x=(v)=>ox+s(v), y=(v)=>oy+s(v);
  const lean=[0,-8,-4,5,-15,-5,-8,-12,10,18][pose];
  ellipse(c,x(64+lean),y(35),s(22),s(25),palette.hair);
  ellipse(c,x(69+lean),y(38),s(17),s(19),palette.skin);
  line(c,x(55+lean),y(62),x(76+lean),y(105),s(14),palette.coat);
  const legSwing=[0,16,-8,6,20,0,4,-5,-12,24][pose];
  line(c,x(62+lean),y(101),x(48-legSwing),y(142),s(8),palette.leg);
  line(c,x(72+lean),y(101),x(80+legSwing),y(142),s(8),palette.leg);
  const arm=[4,15,-8,5,20,35,48,58,-20,-30][pose];
  line(c,x(64+lean),y(70),x(88+arm),y(91-arm/4),s(7),palette.skin);
  line(c,x(57+lean),y(70),x(35-arm/3),y(96+arm/5),s(7),palette.skin);
  const swordX=x(90+arm), swordY=y(90-arm/4);
  line(c,swordX,swordY,swordX+s(38),swordY-s(38+arm/4),s(3),palette.metal);
  line(c,swordX-s(4),swordY-s(4),swordX+s(7),swordY+s(7),s(3),palette.gold);
}

const master = canvas(512,512);
drawCharacter(master,82,35,2.7,0);
const masterPng = encodePng(master.width, master.height, master.pixels);

const sheet = canvas(1280,256);
for (let i=0;i<actions.length;i+=1) {
  rect(sheet,i*128,0,128,8,[40+i*17,65+i*9,105+i*11,255]);
  drawCharacter(sheet,i*128+8,20,0.82,i);
}
const sheetPng = encodePng(sheet.width,sheet.height,sheet.pixels);

const masterPath=path.join(fixtureDir,'master.png'), sheetPath=path.join(fixtureDir,'action-keyframes.png');
await writeFile(masterPath,masterPng); await writeFile(sheetPath,sheetPng);
const sha=(data)=>createHash('sha256').update(data).digest('hex');
const source=await readFile(path.join(fixtureDir,'source.svg'));
const manifestPath=path.join(fixtureDir,'manifest.json');
const manifest=JSON.parse(await readFile(manifestPath,'utf8'));
manifest.sourceSha256=sha(source);
manifest.outputHashes={ 'master.png':sha(masterPng), 'action-keyframes.png':sha(sheetPng) };
await writeFile(manifestPath,`${JSON.stringify(manifest,null,2)}\n`,'utf8');
const hashes=[`${sha(sheetPng)}  action-keyframes.png`,`${sha(masterPng)}  master.png`,`${sha(source)}  source.svg`].join('\n')+'\n';
await writeFile(path.join(fixtureDir,'hashes.sha256'),hashes,'utf8');
process.stdout.write(JSON.stringify({status:'PASS',fixtureId:manifest.fixtureId,actionCount:actions.length,outputHashes:manifest.outputHashes},null,2)+'\n');
