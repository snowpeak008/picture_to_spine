#!/usr/bin/env node
import { build } from 'esbuild';
import { mkdir, rm, writeFile, readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..'),ui=path.join(root,'apps/desktop-ui'),dist=path.join(ui,'dist');
await rm(dist,{recursive:true,force:true});await mkdir(dist,{recursive:true});
await build({entryPoints:[path.join(ui,'src/main.tsx')],outfile:path.join(dist,'app.js'),bundle:true,minify:true,platform:'browser',format:'iife',target:['chrome120'],sourcemap:true,legalComments:'external',loader:{'.css':'css'},entryNames:'app',assetNames:'[name]',logLevel:'info'});
const html='<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src \'self\'; img-src \'self\' data:; style-src \'self\'; script-src \'self\'; connect-src \'none\'"><title>FlashToSpine Production Assist</title><link rel="stylesheet" href="./app.css"></head><body><div id="root"></div><script src="./app.js"></script></body></html>\n';
await writeFile(path.join(dist,'index.html'),html,'utf8');
if(process.argv.includes('--serve')){const types={'.html':'text/html; charset=utf-8','.js':'text/javascript; charset=utf-8','.css':'text/css; charset=utf-8','.map':'application/json'};const server=createServer(async(req,res)=>{try{const rel=req.url==='/'?'index.html':req.url.slice(1);if(rel.includes('..'))throw new Error('invalid path');const file=path.join(dist,rel);const bytes=await readFile(file);res.writeHead(200,{'content-type':types[path.extname(file)]??'application/octet-stream','cache-control':'no-store'});res.end(bytes);}catch{res.writeHead(404);res.end('not found');}});server.listen(1420,'127.0.0.1',()=>process.stdout.write('F2S UI dev server: http://127.0.0.1:1420\n'));}
