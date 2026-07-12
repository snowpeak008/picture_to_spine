#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root=process.cwd();
const outArg=process.argv.find((v)=>v.startsWith('--output='));
const outPath=path.resolve(root,outArg?.slice(9)??'evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-01/minimal.psd');
const reportPath=path.join(path.dirname(outPath),'psd-roundtrip.json');
const hashPath=path.join(path.dirname(outPath),'minimal.psd.sha256');
const width=64,height=64;
const makeLayer=(name,pixel)=>({name,pixel});
const layers=[makeLayer('body',[64,104,168,255]),makeLayer('weapon',[217,227,239,150])];
const u16=(n)=>{const b=Buffer.alloc(2);b.writeUInt16BE(n);return b;},i16=(n)=>{const b=Buffer.alloc(2);b.writeInt16BE(n);return b;},u32=(n)=>{const b=Buffer.alloc(4);b.writeUInt32BE(n);return b;},i32=(n)=>{const b=Buffer.alloc(4);b.writeInt32BE(n);return b;};
function pascal4(name){const text=Buffer.from(name,'ascii');const len=Math.min(text.length,255);const total=Math.ceil((1+len)/4)*4;const b=Buffer.alloc(total);b[0]=len;text.copy(b,1,0,len);return b;}
function record(layer){
  const channelLen=2+width*height;
  const channels=Buffer.concat([i16(0),u32(channelLen),i16(1),u32(channelLen),i16(2),u32(channelLen),i16(-1),u32(channelLen)]);
  const extra=Buffer.concat([u32(0),u32(0),pascal4(layer.name)]);
  return Buffer.concat([i32(0),i32(0),i32(height),i32(width),u16(4),channels,Buffer.from('8BIMnorm','ascii'),Buffer.from([255,0,0,0]),u32(extra.length),extra]);
}
function channelData(layer){return Buffer.concat(layer.pixel.map((value)=>Buffer.concat([u16(0),Buffer.alloc(width*height,value)])));}
const records=Buffer.concat(layers.map(record));const pixels=Buffer.concat(layers.map(channelData));let layerInfo=Buffer.concat([i16(layers.length),records,pixels]);if(layerInfo.length%2)layerInfo=Buffer.concat([layerInfo,Buffer.alloc(1)]);
const layerMaskData=Buffer.concat([u32(layerInfo.length),layerInfo,u32(0)]);
const header=Buffer.alloc(26);header.write('8BPS',0,'ascii');header.writeUInt16BE(1,4);header.writeUInt16BE(4,12);header.writeUInt32BE(height,14);header.writeUInt32BE(width,18);header.writeUInt16BE(8,22);header.writeUInt16BE(3,24);
const composite=Buffer.concat([u16(0),...Array.from({length:4},(_,channel)=>Buffer.alloc(width*height,layers[0].pixel[channel]))]);
const psd=Buffer.concat([header,u32(0),u32(0),u32(layerMaskData.length),layerMaskData,composite]);

function parse(buffer){
  const issues=[];if(buffer.subarray(0,4).toString('ascii')!=='8BPS')issues.push('BAD_SIGNATURE');if(buffer.readUInt16BE(4)!==1)issues.push('BAD_VERSION');
  const parsed={channels:buffer.readUInt16BE(12),height:buffer.readUInt32BE(14),width:buffer.readUInt32BE(18),depth:buffer.readUInt16BE(22),colorMode:buffer.readUInt16BE(24)};
  let offset=26;offset+=4+buffer.readUInt32BE(offset);offset+=4+buffer.readUInt32BE(offset);const lmLength=buffer.readUInt32BE(offset);offset+=4;const lmEnd=offset+lmLength;const layerLength=buffer.readUInt32BE(offset);offset+=4;const layerEnd=offset+layerLength;parsed.layerCount=Math.abs(buffer.readInt16BE(offset));offset+=2;parsed.layerNames=[];
  for(let i=0;i<parsed.layerCount;i+=1){offset+=16;const count=buffer.readUInt16BE(offset);offset+=2+count*6;offset+=12;const extraLength=buffer.readUInt32BE(offset);offset+=4;const extraEnd=offset+extraLength;const maskLen=buffer.readUInt32BE(offset);offset+=4+maskLen;const rangeLen=buffer.readUInt32BE(offset);offset+=4+rangeLen;const nameLen=buffer[offset];parsed.layerNames.push(buffer.subarray(offset+1,offset+1+nameLen).toString('ascii'));offset=extraEnd;}
  if(offset>layerEnd||layerEnd>lmEnd||lmEnd>buffer.length)issues.push('TRUNCATED_LAYER_DATA');
  if(parsed.layerCount!==2||parsed.layerNames.join(',')!=='body,weapon')issues.push('LAYER_MISMATCH');if(parsed.width!==width||parsed.height!==height||parsed.channels!==4)issues.push('DIMENSION_MISMATCH');
  return {parsed,issues};
}
const roundtrip=parse(psd),hash=createHash('sha256').update(psd).digest('hex');await mkdir(path.dirname(outPath),{recursive:true});await writeFile(outPath,psd);await writeFile(hashPath,`${hash}  minimal.psd\n`,'utf8');
const report={schemaVersion:'1.0.0',status:roundtrip.issues.length?'FAIL':'PASS',writer:'f2s-minimal-psd-v1',license:'workspace-owned-proprietary',sha256:hash,...roundtrip};await writeFile(reportPath,`${JSON.stringify(report,null,2)}\n`,'utf8');process.stdout.write(JSON.stringify(report,null,2)+'\n');if(roundtrip.issues.length)process.exitCode=2;
