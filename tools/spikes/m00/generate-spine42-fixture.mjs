#!/usr/bin/env node
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { deflateSync } from 'node:zlib';
import path from 'node:path';

const root=process.cwd(), dir=path.join(root,'fixtures/m00/spine42-probe'), attachments=path.join(dir,'attachments');
const sha=(data)=>createHash('sha256').update(data).digest('hex');
function crc32(buffer){let crc=0xffffffff;for(const byte of buffer){crc^=byte;for(let i=0;i<8;i+=1)crc=(crc>>>1)^(0xedb88320&-(crc&1));}return(crc^0xffffffff)>>>0;}
function chunk(type,data){const name=Buffer.from(type),out=Buffer.alloc(12+data.length);out.writeUInt32BE(data.length,0);name.copy(out,4);data.copy(out,8);out.writeUInt32BE(crc32(Buffer.concat([name,data])),8+data.length);return out;}
function png(width,height,pixels){const h=Buffer.alloc(13);h.writeUInt32BE(width,0);h.writeUInt32BE(height,4);h[8]=8;h[9]=6;const raw=Buffer.alloc(height*(1+width*4));for(let y=0;y<height;y+=1)pixels.copy(raw,y*(1+width*4)+1,y*width*4,(y+1)*width*4);return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',h),chunk('IDAT',deflateSync(raw,{level:9})),chunk('IEND',Buffer.alloc(0))]);}
await mkdir(attachments,{recursive:true});
const body=await readFile(path.join(root,'fixtures/m00/synthetic-character/master.png'));
const weaponPixels=Buffer.alloc(128*128*4);
for(let y=0;y<128;y+=1)for(let x=0;x<128;x+=1){const i=(y*128+x)*4;const blade=Math.abs((127-y)-x)<5&&x>18;const guard=Math.abs(x-32)<4&&y>78&&y<118;if(blade||guard){weaponPixels[i]=blade?220:197;weaponPixels[i+1]=blade?230:154;weaponPixels[i+2]=blade?240:69;weaponPixels[i+3]=255;}}
const weapon=png(128,128,weaponPixels);
await writeFile(path.join(attachments,'body.png'),body);await writeFile(path.join(attachments,'weapon.png'),weapon);
const rigBytes=await readFile(path.join(dir,'rig-ir.json')), skeletonBytes=await readFile(path.join(dir,'skeleton.json'));
const rig=JSON.parse(rigBytes), skeleton=JSON.parse(skeletonBytes);
const finite=(value)=>typeof value!=='number'||Number.isFinite(value);
function walk(value){if(Array.isArray(value))return value.every(walk);if(value&&typeof value==='object')return Object.values(value).every(walk);return finite(value);}
const issues=[];
if(skeleton.skeleton?.spine!=='4.2.43')issues.push('SPINE_PATCH_MISMATCH');
if(rig.timeBase?.numerator!==1||rig.timeBase?.denominator!==30000)issues.push('TIMEBASE_MISMATCH');
if(!walk(rig)||!walk(skeleton))issues.push('NON_FINITE_NUMBER');
for(const p of ['body.png','weapon.png'])if(p.includes('..')||path.isAbsolute(p))issues.push('PATH_NOT_RELATIVE');
const manifestPath=path.join(dir,'capability-manifest.json'),manifest=JSON.parse(await readFile(manifestPath,'utf8'));
manifest.sourceHashes={ 'rig-ir.json':sha(rigBytes), 'skeleton.json':sha(skeletonBytes) };
manifest.fixtureHashes={ 'attachments/body.png':sha(body), 'attachments/weapon.png':sha(weapon) };
manifest.staticContractStatus=issues.length?'FAILED':'VERIFIED';
manifest.editorRoundTripStatus='EXTERNAL';
await writeFile(manifestPath,`${JSON.stringify(manifest,null,2)}\n`,'utf8');
const report={schemaVersion:'1.0.0',status:issues.length?'FAIL':'PASS',capabilityId:manifest.capabilityId,issues,sourceHashes:manifest.sourceHashes,fixtureHashes:manifest.fixtureHashes,editorRoundTripStatus:'EXTERNAL'};
process.stdout.write(JSON.stringify(report,null,2)+'\n');if(issues.length)process.exitCode=2;
