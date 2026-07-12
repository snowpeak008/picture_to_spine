#!/usr/bin/env node
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
const roots=['apps','crates','tools'];const findings=[];
async function walk(dir){for(const e of await readdir(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory()){if(!['node_modules','target','dist'].includes(e.name))await walk(p);}else if(/\.(rs|ts|tsx|mjs|json|toml|ps1|css)$/.test(e.name)){const text=await readFile(p,'utf8');if(text.charCodeAt(0)===0xfeff)findings.push({code:'UTF8_BOM',file:p});text.split(/\r?\n/).forEach((line,index)=>{if(/[ \t]+$/.test(line))findings.push({code:'TRAILING_WHITESPACE',file:p,line:index+1});});}}}
for(const root of roots)await walk(root);process.stdout.write(JSON.stringify({status:findings.length?'FAIL':'PASS',findings},null,2)+'\n');if(findings.length)process.exitCode=2;
