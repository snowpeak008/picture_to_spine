#!/usr/bin/env node
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const root=process.cwd(),rules=JSON.parse(await readFile(path.join(root,'tools/architecture/dependency-rules.json'),'utf8')),findings=[];
for(const [layer,file] of [['domain','crates/domain/Cargo.toml'],['application','crates/application/Cargo.toml']]){
  const text=await readFile(path.join(root,file),'utf8');
  for(const denied of rules.rust[`${layer}Forbidden`])if(new RegExp(`(^|\\n)${denied.replace('-','\\-')}\\s*=`,`m`).test(text))findings.push({code:'RUST_REVERSE_DEPENDENCY',layer,dependency:denied,file});
}
async function walk(dir){const out=[];for(const entry of await readdir(dir,{withFileTypes:true})){const p=path.join(dir,entry.name);if(entry.isDirectory())out.push(...await walk(p));else if(/\.(ts|tsx)$/.test(entry.name))out.push(p);}return out;}
const uiRoot=path.join(root,'apps/desktop-ui/src');
for(const file of await walk(uiRoot)){
  const rel=path.relative(root,file).replaceAll('\\','/'),text=await readFile(file,'utf8');
  for(const denied of rules.typescript.forbiddenAnywhere)if(text.includes(denied))findings.push({code:'FORBIDDEN_UI_CAPABILITY',file:rel,dependency:denied});
  if(text.includes('@tauri-apps/api')&&!rules.typescript.tauriApiAllowedRoots.some(prefix=>rel.startsWith(prefix)))findings.push({code:'TAURI_API_OUTSIDE_SERVICE',file:rel});
}
const result={schemaVersion:'1.0.0',status:findings.length?'FAIL':'PASS',findings:findings.toSorted((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b)))};
process.stdout.write(JSON.stringify(result,null,2)+'\n');if(findings.length)process.exitCode=2;
