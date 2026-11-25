import fs from 'node:fs';
import {execSync} from 'node:child_process';
const ws = new WebSocket('ws://127.0.0.1:8112/ws');
let next=1; const pending=new Map();
function call(tag,payload,timeoutMs=5000){return new Promise((resolve,reject)=>{const id=String(9300000000+next++);pending.set(id,{resolve,reject,tag});ws.send(JSON.stringify({_tag:'Request',id,tag,payload,headers:[]}));setTimeout(()=>{if(pending.has(id)){pending.delete(id);reject(new Error('timeout '+tag))}},timeoutMs)})}
ws.onmessage=(ev)=>{const m=JSON.parse(ev.data); if(m._tag==='Exit'){const p=pending.get(String(m.requestId)); if(!p)return; pending.delete(String(m.requestId)); if(m.exit?._tag==='Success')p.resolve(m.exit.value); else p.reject(new Error(p.tag+': '+JSON.stringify(m.exit)))}}
ws.onerror=()=>{console.error('ws error');process.exit(2)};
await new Promise((res,rej)=>{ws.onopen=res;setTimeout(()=>rej(new Error('open timeout')),3000)});
const sleep=(ms)=>new Promise(r=>setTimeout(r,ms));
const suffix=Date.now().toString(36); const projectId='project-oc-'+suffix, threadId='thread-oc-'+suffix;
fs.mkdirSync('/tmp/oc-workspace-'+suffix,{recursive:true});
const now=new Date().toISOString();
await call('orchestration.dispatchCommand',{type:'project.create',commandId:'cmd-project-'+suffix,projectId,title:'OC test',workspaceRoot:'/tmp/oc-workspace-'+suffix,createdAt:now});
await call('orchestration.dispatchCommand',{type:'thread.create',commandId:'cmd-thread-'+suffix,threadId,projectId,title:'Thread',createdAt:now,envMode:'local',runtimeMode:'approval-required',modelSelection:{provider:'opencode',model:'test'}});
await call('orchestration.dispatchCommand',{type:'thread.turn.start',commandId:'cmd-turn-'+suffix,threadId,createdAt:new Date().toISOString(),dispatchMode:'queue',runtimeMode:'approval-required',modelSelection:{provider:'opencode',model:'test'},message:{messageId:'msg-'+suffix,text:'hello'}});
let rid='';
for(let i=0;i<30;i++){
 await sleep(250);
 try { rid=execSync(`sqlite3 /tmp/synara-oc-home/userdata/state.sqlite "select request_id from projection_pending_interactions where thread_id='${threadId}' and status='pending' and interaction_kind='userInput' limit 1"`).toString().trim(); } catch {}
 if(rid) break;
}
if(!rid){ console.error('no pending form'); process.exit(3) }
await call('orchestration.dispatchCommand',{type:'thread.user-input.respond',commandId:'cmd-input-'+suffix,threadId,requestId:rid,answers:{'question-0-choice':'Visible','question-1-confirm':'true'},createdAt:new Date().toISOString()});
await sleep(1300);
const detail=await call('orchestration.getThreadDetailSnapshot',{threadId});
const thread=detail.thread??detail;
console.log(JSON.stringify({threadId,session:thread.session,messages:(thread.messages??[]).map(m=>({role:m.role,text:m.text}))},null,2));
ws.close();
