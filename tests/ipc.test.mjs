import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
const rust=fs.readFileSync('src-tauri/src/lib.rs','utf8');
const commands=new Map([...rust.matchAll(/#\[tauri::command\]\s*(?:async\s+)?fn\s+(\w+)\s*\(([\s\S]*?)\)\s*->/g)].map(m=>[m[1], [...m[2].matchAll(/\b(\w+)\s*:\s*([^,\n]+)/g)].filter(p=>!['state','app'].includes(p[1])).map(p=>({name:p[1].replace(/_([a-z])/g,(_,c)=>c.toUpperCase()), optional:p[2].includes('Option<')}))]));
function files(dir){return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?files(path.join(dir,e.name)):/\.tsx?$/.test(e.name)?[path.join(dir,e.name)]:[]);}
test('all frontend commands and required argument names match the registered Rust API',()=>{
  let calls=0;
  for(const file of files('src')){
    const source=ts.createSourceFile(file,fs.readFileSync(file,'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
    const visit=node=>{
      if(ts.isCallExpression(node)&&ts.isIdentifier(node.expression)&&node.expression.text==='invoke'&&ts.isStringLiteral(node.arguments[0])){
        const name=node.arguments[0].text;calls++;
        assert(commands.has(name),`${file}: unknown command ${name}`);
        assert(rust.slice(rust.indexOf('tauri::generate_handler![')).includes(name+','),`${name} is not registered`);
        const args=node.arguments[1];
        const names=args&&ts.isObjectLiteralExpression(args)?new Set(args.properties.map(p=>p.name?.getText(source).replace(/^['"]|['"]$/g,''))):new Set();
        for(const p of commands.get(name)){if(!p.optional)assert(names.has(p.name),`${file}: ${name} missing required ${p.name}`);}
      }
      ts.forEachChild(node,visit);
    };visit(source);
  }
  assert(calls>=30);
});
test('frontend streaming event names are emitted by the backend',()=>{
  const text=fs.readFileSync('src/components/AiAssistant.tsx','utf8');
  const events=[];
  const source=ts.createSourceFile('chat.tsx',text,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
  const visit=node=>{if(ts.isCallExpression(node)&&ts.isIdentifier(node.expression)&&node.expression.text==='listen'&&ts.isStringLiteral(node.arguments[0]))events.push(node.arguments[0].text);ts.forEachChild(node,visit);};visit(source);
  assert(events.length>=4);
  for(const event of events)assert(new RegExp(`emit\\s*\\(\\s*"${event}"`).test(rust),`Backend never emits ${event}`);
});
