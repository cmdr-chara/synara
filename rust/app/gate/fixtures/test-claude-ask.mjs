import {execSync} from 'node:child_process';
import fs from 'node:fs';
const ws = new WebSocket('ws://127.0.0.1:8113/ws');
let next=1; const pending=new Map();
function call(tag,payload,timeoutMs=5000){return new Promise((resolve,reject)=>{const id=String(9500000000+next++);pending.set(id,{resolve,reject,tag});ws.send(JSON.stringify({_tag:'Request',id,tag,payload,headers:[]}));setTimeout(()=>{if(pending.has(id)){pending.delete(id);reject(new Error('timeout '+tag))}},timeoutMs)})}
ws.onmessage=(ev)=>{const m=JSON.parse(ev.data); if(m._tag==='Exit'){const p=pending.get(String(m.requestId)); if(!p)return; pending.delete(String(m.requestId)); if(m.exit?._tag==='Success')p.resolve(m.exit.value); else p.reject(new Error(p.tag+': '+JSON.stringify(m.exit)))}}
await new Promise((res,rej)=>{ws.onopen=res;setTimeout(()=>rej(new Error('open timeout')),3000)});
const sleep=(ms)=>new Promise(r=>setTimeout(r,ms));
const suffix=Date.now().toString(36); const projectId='project-cl-'+suffix, threadId='thread-cl-'+suffix, root='/tmp/cl-workspace-'+suffix;
fs.mkdirSync(root,{recursive:true});
const now=new Date().toISOString();
await call('orchestration.dispatchCommand',{type:'project.create',commandId:'cmd-project-'+suffix,projectId,title:'CL test',workspaceRoot:root,createdAt:now});
await call('orchestration.dispatchCommand',{type:'thread.create',commandId:'cmd-thread-'+suffix,threadId,projectId,title:'Thread',createdAt:now,envMode:'local',runtimeMode:'full-access',modelSelection:{provider:'claudeAgent',model:'test'}});
await call('orchestration.dispatchCommand',{type:'thread.turn.start',commandId:'cmd-turn-'+suffix,threadId,createdAt:new Date().toISOString(),dispatchMode:'queue',runtimeMode:'full-access',modelSelection:{provider:'claudeAgent',model:'test'},message:{messageId:'msg-'+suffix,text:'hello'}});
let rid='';
for(let i=0;i<30;i++){
 await sleep(250);
 try { rid=execSync(`sqlite3 /tmp/synara-cl-home/userdata/state.sqlite "select request_id from projection_pending_interactions where thread_id='${threadId}' and status='pending' and interaction_kind='userInput' limit 1"`).toString().trim(); } catch {}
 if(rid) break;
}
if(!rid){ console.error('no pending ask'); process.exit(3) }
await call('orchestration.dispatchCommand',{type:'thread.user-input.respond',commandId:'cmd-input-'+suffix,threadId,requestId:rid,answers:{'Pick':'Visible'},createdAt:new Date().toISOString()});
await sleep(1300);
const state=execSync(`sqlite3 /tmp/synara-cl-home/userdata/state.sqlite "select state from projection_turns where thread_id='${threadId}'"`).toString().trim();
const text=execSync(`sqlite3 /tmp/synara-cl-home/userdata/state.sqlite "select text from projection_messages where thread_id='${threadId}' and role='assistant' order by created_at desc limit 1"`).toString().trim();
console.log(JSON.stringify({threadId,state,text,control:fs.existsSync('/tmp/fake-claude-ask-log')?fs.readFileSync('/tmp/fake-claude-ask-log','utf8'):''},null,2));
ws.close();
