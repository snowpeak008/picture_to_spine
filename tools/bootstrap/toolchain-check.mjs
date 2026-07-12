#!/usr/bin/env node
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';

const root=process.cwd(),expected={node:'24.15.0',npm:'11.12.1',rustc:'1.96.0',cargo:'1.96.0',python:'3.12.4',uv:'0.11.8'};
const commands={node:['node',['--version']],npm:[process.env.ComSpec??'cmd.exe',['/d','/c','npm.cmd','--version']],rustc:['rustc',['--version']],cargo:['cargo',['--version']],python:['python',['--version']],uv:['uv',['--version']]};
const findings=[],observed={};
for(const [id,[exe,args]] of Object.entries(commands)){try{const raw=execFileSync(exe,args,{encoding:'utf8'}).trim();const match=raw.match(/\d+\.\d+\.\d+/);observed[id]=match?.[0]??raw;if(observed[id]!==expected[id])findings.push({code:'EXACT_VERSION_MISMATCH',id,expected:expected[id],actual:observed[id]});}catch(error){findings.push({code:'TOOL_MISSING_OR_FAILED',id,message:error.message});}}
const packageJson=JSON.parse(await readFile(path.join(root,'package.json'),'utf8'));
if(packageJson.packageManager!=='npm@11.12.1')findings.push({code:'PACKAGE_MANAGER_NOT_PINNED'});
for(const text of [JSON.stringify(packageJson),await readFile(path.join(root,'rust-toolchain.toml'),'utf8')])if(/latest|[~^]\d/.test(text))findings.push({code:'FLOATING_TOOLCHAIN_VERSION'});
let probeState='NOT_RUN';try{probeState=JSON.parse(await readFile(path.join(root,'evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-01/probe.json'),'utf8')).overallState;}catch{}
const report={schemaVersion:'1.0.0',status:findings.length?'FAIL':'PASS',capabilityState:findings.length?'FAILED':'UNVERIFIED_CLEAN_VM',expected,observed,probeState,findings};
const out=path.join(root,'evidence/M01/F2S-DEV-M01-002/F2S-WU-M01-002-03/toolchain-check.json');await mkdir(path.dirname(out),{recursive:true});await writeFile(out,`${JSON.stringify(report,null,2)}\n`,'utf8');process.stdout.write(JSON.stringify(report,null,2)+'\n');if(findings.length)process.exitCode=2;
