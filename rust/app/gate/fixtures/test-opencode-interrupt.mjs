import {execSync} from 'node:child_process';
import fs from 'node:fs';
const ws = new WebSocket('ws://127.0.0.1:8112/ws');
let next=1; const pending=new Map();
function call(tag,payload,timeoutMs=5000){return new Promise((resolve,reject)=>{const id=String(9400000000+next++);pending.set(id,{resolve,reject,tag});ws.send(JSON.stringify({_tag:'Request',id,tag,payload,headers:[]}));setTimeout(()=>{if(pending.has(id)){pending.delete(id);reject(new Error('timeout '+tag))}},timeoutMs)})}
ws.onmessage=(ev)=>{const m=JSON.parse(ev.data); if(m._tag==='Exit'){const p=pending.get(String(m.requestId)); if(!p)return; pending.delete(String(m.requestId)); if(m.exit?._tag==='Success')p.resolve(m.exit.value); else p.reject(new Error(p.tag+': '+JSON.stringify(m.exit)))}}
await new Promise((res,rej)=>{ws.onopen=res;setTimeout(()=>rej(new Error('open timeout')),3000)});
const sleep=(ms)=>new Promise(r=>setTimeout(r,ms));
const suffix=Date.now().toString(36); const projectId='project-oc-'+suffix, threadId='thread-oc-'+suffix;
const now=new Date().toISOString(); fs.mkdirSync('/tmp/oc-workspace-'+suffix,{recursive:true});
await call('orchestration.dispatchCommand',{type:'project.create',commandId:'cmd-project-'+suffix,projectId,title:'OC test',workspaceRoot:'/tmp/oc-workspace-'+suffix,createdAt:now});
await call('orchestration.dispatchCommand',{type:'thread.create',commandId:'cmd-thread-'+suffix,threadId,projectId,title:'Thread',createdAt:now,envMode:'local',runtimeMode:'approval-required',modelSelection:{provider:'opencode',model:'test'}});
await call('orchestration.dispatchCommand',{type:'thread.turn.start',commandId:'cmd-turn-'+suffix,threadId,createdAt:new Date().toISOString(),dispatchMode:'queue',runtimeMode:'approval-required',modelSelection:{provider:'opencode',model:'test'},message:{messageId:'msg-'+suffix,text:'hello'}});
await sleep(1000);
await call('orchestration.dispatchCommand',{type:'thread.turn.interrupt',commandId:'cmd-interrupt-'+suffix,threadId,createdAt:new Date().toISOString()});
await sleep(1500);
const state=execSync(`sqlite3 /tmp/synara-oc-home/userdata/state.sqlite "select state from projection_turns where thread_id='${threadId}'"`).toString().trim();
console.log(JSON.stringify({threadId,state},null,2));
ws.close();
