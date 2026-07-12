#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readFile, readdir, stat, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import process from 'node:process';

const args=Object.fromEntries(process.argv.slice(2).map(value=>{const i=value.indexOf('=');return i<0?[value.replace(/^--/,''),true]:[value.slice(2,i),value.slice(i+1)];}));
if(!/^F2S-DEV-M\d{2}-\d{3}$/.test(args.task??''))throw new Error('exact --task is required');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
async function collect(rel,out={}){const full=path.resolve(rel);let info;try{info=await stat(full);}catch{return out}if(info.isDirectory()){for(const name of (await readdir(full)).sort())await collect(path.join(rel,name),out);}else out[rel.replaceAll('\\','/')]=sha(await readFile(full));return out;}
const outputs={};for(const item of (args.outputs??'').split(';').filter(Boolean))await collect(item,outputs);
const inputs={};for(const item of (args.inputs??'').split(';').filter(Boolean))await collect(item,inputs);
const status=args.status??'PASS',capabilityState=args.capability??'UNVERIFIED',now=new Date().toISOString();
const envelope={schemaVersion:'1.0.0',evidenceId:`F2S-EVD-${args.task.slice(8)}`,taskId:args.task,status,command:args.command??'manual implementation verification',exitCode:status==='NOT_RUN'?null:status==='PASS'?0:1,startedAtUtc:now,endedAtUtc:now,runner:{kind:'local-development',os:`${os.platform()} ${os.release()}`,architecture:os.arch(),hostnameHash:sha(Buffer.from(os.hostname()))},toolVersions:{node:process.version},inputHashes:inputs,outputHashes:outputs,logRefs:(args.logs??'').split(';').filter(Boolean),reportRefs:(args.reports??'').split(';').filter(Boolean),capabilityState,externalBlockers:(args.blockers??'').split(';').filter(Boolean),previousEvidenceRef:null,notRunReason:status==='NOT_RUN'?(args.reason??'External execution was not available.'):null,payload:{summary:args.summary??'',generatedBy:'tools/evidence/create-envelope.mjs'}};
if(capabilityState==='EXTERNAL'&&!envelope.externalBlockers.length)throw new Error('EXTERNAL requires --blockers');
const out=path.resolve(args.output??`evidence/${args.task.slice(8,11)}/${args.task}/evidence.json`);await mkdir(path.dirname(out),{recursive:true});await writeFile(out,`${JSON.stringify(envelope,null,2)}\n`,'utf8');process.stdout.write(`${out}\n`);
