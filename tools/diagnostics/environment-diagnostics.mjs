#!/usr/bin/env node
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
const root=process.cwd();
async function json(rel,fallback){try{return JSON.parse(await readFile(path.join(root,rel),'utf8'));}catch{return fallback;}}
const probe=await json('evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-01/probe.json',{tools:[],overallState:'NOT_RUN'});
const sandbox=await json('evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-01/sandbox-probe.json',{capabilityState:'UNVERIFIED'});
const gpu=await json('evidence/M00/F2S-DEV-M00-004/F2S-WU-M00-004-02/gpu-matrix.json',{capabilityState:'UNVERIFIED',gpu:[]});
const spine=await json('fixtures/m00/spine42-probe/capability-manifest.json',{staticContractStatus:'UNVERIFIED',editorRoundTripStatus:'EXTERNAL'});
const capabilities=[...probe.tools.map(tool=>({id:tool.id,state:tool.state,version:tool.version,diagnosticCode:tool.state==='MISSING'?'F2S-ENV-MISSING':'F2S-ENV-OBSERVED'})),{id:'windows-appcontainer-v1',state:sandbox.capabilityState,diagnosticCode:'F2S-SANDBOX-UNVERIFIED'},{id:'gpu-worker',state:gpu.capabilityState,version:gpu.gpu?.[0]?.name??null,diagnosticCode:'F2S-GPU-UNVERIFIED'},{id:'spine-static-4.2.43',state:spine.staticContractStatus,version:'4.2.43',diagnosticCode:'F2S-SPINE-STATIC'},{id:'spine-editor-roundtrip',state:spine.editorRoundTripStatus,version:'4.2.43',diagnosticCode:'F2S-SPINE-EXTERNAL'}];
const report={schemaVersion:'1.0.0',generatedAtUtc:new Date().toISOString(),capabilities,redaction:{paths:'tool executable paths omitted from exported DTO',credentials:'never read',userImages:'never read'},networkCallCount:0};
const out=path.join(root,'evidence/M01/F2S-DEV-M01-005/F2S-WU-M01-005-01/environment-diagnostics.json');await mkdir(path.dirname(out),{recursive:true});await writeFile(out,`${JSON.stringify(report,null,2)}\n`,'utf8');process.stdout.write(JSON.stringify(report,null,2)+'\n');
