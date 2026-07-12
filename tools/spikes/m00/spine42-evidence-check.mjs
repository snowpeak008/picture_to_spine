#!/usr/bin/env node
import { readFile, writeFile, mkdir, stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import process from 'node:process';

const root=process.cwd();
const leaseArg=process.argv.find((value)=>value.startsWith('--lease='));
const outputArg=process.argv.find((value)=>value.startsWith('--output='));
const output=path.resolve(root,outputArg?.slice(9)??'evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-03/editor-roundtrip.json');
const issues=[];let lease=null,status='NOT_RUN',capabilityState='EXTERNAL',notRunReason='No user-approved Spine 4.2.43 evidence lease was supplied.';
if(leaseArg){
  const leasePath=path.resolve(root,leaseArg.slice(8));
  try{lease=JSON.parse(await readFile(leasePath,'utf8'));}catch(error){issues.push({code:'LEASE_INVALID',message:error.message});}
  if(lease){
    if(lease.approved!==true)issues.push({code:'LEASE_NOT_APPROVED'});
    if(lease.exactVersion!=='4.2.43')issues.push({code:'SPINE_PATCH_MISMATCH',actual:lease.exactVersion});
    if(lease.licenseConfirmed!==true)issues.push({code:'LICENSE_NOT_CONFIRMED'});
    if(!lease.expiresAtUtc||Date.parse(lease.expiresAtUtc)<=Date.now())issues.push({code:'LEASE_EXPIRED'});
    for(const item of lease.outputs??[]){
      const p=path.resolve(item.path);if(!p.startsWith(path.resolve(lease.allowedOutputRoot)+path.sep))issues.push({code:'OUTPUT_ESCAPES_ROOT',path:item.path});
      try{const bytes=await readFile(p);const actual=createHash('sha256').update(bytes).digest('hex');if(actual!==item.sha256)issues.push({code:'OUTPUT_HASH_MISMATCH',path:item.path});await stat(p);}catch(error){issues.push({code:'OUTPUT_MISSING',path:item.path,message:error.message});}
    }
    status=issues.length?'FAIL':'PASS';capabilityState=issues.length?'FAILED':'VERIFIED';notRunReason=null;
  }
}
const report={schemaVersion:'1.0.0',evidenceType:'F2S-SPINE42-COMPAT-EVIDENCE-001',status,capabilityState,exactVersion:'4.2.43',observedAtUtc:new Date().toISOString(),leaseId:lease?.leaseId??null,writerProvenance:lease?.writerProvenance??'EXTERNAL_USER_TOOL',outputs:lease?.outputs??[],issues,notRunReason,externalBlockers:status==='NOT_RUN'?['User legal Spine Professional/eligible Enterprise 4.2.43 and explicit lease']:[]};
await mkdir(path.dirname(output),{recursive:true});await writeFile(output,`${JSON.stringify(report,null,2)}\n`,'utf8');process.stdout.write(JSON.stringify(report,null,2)+'\n');if(status==='FAIL')process.exitCode=2;
