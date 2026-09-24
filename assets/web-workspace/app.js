'use strict';
const token=document.querySelector('#token');
const load=document.querySelector('#load');
const addWorkspace=document.querySelector('#add-workspace');
const createTask=document.querySelector('#create-task');
const root=document.querySelector('#root');
const project=document.querySelector('#project');
const profile=document.querySelector('#profile');
const titleInput=document.querySelector('#title');
const draft=document.querySelector('#draft');
const status=document.querySelector('#status');
const tasks=document.querySelector('#tasks');
const thread=document.querySelector('#thread');
let selectedTask=0;
let activeDraftEditor=null;
let activeDraftSaved='';
let pollTimer=null;
const QUESTION_DRAFT_PREFIX='synara-question-draft-v1:';
const QUESTION_DRAFT_TTL=24*60*60*1000;
function questionDraftKey(base,id){return QUESTION_DRAFT_PREFIX+base+':'+id}
function clearQuestionDraft(base,id){try{localStorage.removeItem(questionDraftKey(base,id))}catch(_){/* Storage may be disabled. */}}
function readQuestionDraft(base,item){
  try{
    const raw=localStorage.getItem(questionDraftKey(base,item.id));if(!raw)return null;
    const value=JSON.parse(raw);
    if(value.expires<Date.now()||value.schema!==JSON.stringify(item.fields)){clearQuestionDraft(base,item.id);return null}
    return value.fields;
  }catch(_){return null}
}
function saveQuestionDraft(base,item,fields){
  try{
    const value={expires:Date.now()+QUESTION_DRAFT_TTL,schema:JSON.stringify(item.fields),fields};
    const encoded=JSON.stringify(value);
    if(new TextEncoder().encode(encoded).length>65536)return false;
    localStorage.setItem(questionDraftKey(base,item.id),encoded);return true;
  }catch(_){return false}
}
function pruneQuestionDrafts(){
  try{
    for(let index=localStorage.length-1;index>=0;index--){
      const key=localStorage.key(index);if(!key||!key.startsWith(QUESTION_DRAFT_PREFIX))continue;
      try{if(JSON.parse(localStorage.getItem(key)).expires<Date.now())localStorage.removeItem(key)}
      catch(_){localStorage.removeItem(key)}
    }
  }catch(_){/* Storage may be disabled. */}
}
function discardUnsaved(){return !activeDraftEditor||activeDraftEditor.value===activeDraftSaved||confirm('Discard unsaved draft changes?')}
function retireSelection(){clearTimeout(pollTimer);pollTimer=null;activeDraftEditor=null;return ++selectedTask}
function message(error,fallback){return error instanceof Error?error.message:fallback}
async function requestJson(path,body){
  const options={headers:{Authorization:`Bearer ${token.value}`},cache:'no-store'};
  if(body!==undefined){options.method='POST';options.headers['Content-Type']='application/json';options.body=JSON.stringify(body)}
  const response=await fetch(path,options);
  const data=await response.json();
  if(!response.ok){
    const errors={unauthorized:'The token was not accepted.',interaction_expired:'This request is no longer active. Refresh to review current requests.',invalid_interaction_reply:'The response does not match the current request. Check required fields and offered choices.',draft_changed:'The saved draft changed in another tab. Reload and review it before running.',route_changed:'The selected model or agent route changed. Refresh and review it before running.',workspace_changed:'The SSH workspace connection changed. Refresh and review the destination before running.',run_active:'This task already has an active web run.',authentication_required:'Sign in with the agent locally before running it here.'};
    throw new Error(errors[data.error]||`Server returned ${response.status} (${data.error||'request failed'}).`);
  }
  return data;
}
const getJson=path=>requestJson(path);
const postJson=(path,body)=>requestJson(path,body);
function fillSelect(select,items,label){
  select.replaceChildren();
  for(const item of items){const option=document.createElement('option');option.value=item.id;option.textContent=item.name;select.append(option)}
  if(items.length===0){const option=document.createElement('option');option.textContent=label;option.value='';select.append(option)}
}
function setBusy(busy){for(const button of [load,addWorkspace,createTask])button.disabled=busy}
function button(text,handler){const result=document.createElement('button');result.type='button';result.textContent=text;result.addEventListener('click',handler);return result}
async function loadWorkspace(selectedId=null){
  if(!discardUnsaved())return false;
  pruneQuestionDrafts();
  const selection=retireSelection();
  setBusy(true);status.textContent='Loading…';tasks.replaceChildren();thread.replaceChildren();
  try{
    const [catalog,profiles]=await Promise.all([getJson('/api/catalog'),getJson('/api/profiles')]);
    if(selection!==selectedTask)return false;
    fillSelect(project,catalog.projects,'Add a local folder first');
    fillSelect(profile,profiles.profiles,'No agent profiles available');
    for(const task of catalog.tasks)tasks.append(button(task.title,()=>showThread(task)));
    status.textContent=catalog.truncated.tasks?'Showing the first 32 tasks.':`${catalog.tasks.length} tasks loaded.`;
    if(catalog.tasks.length===0){const empty=document.createElement('p');empty.textContent='No tasks yet.';tasks.append(empty)}
    else await showThread(catalog.tasks.find(task=>task.id===selectedId)||catalog.tasks[0]);
    return true;
  }catch(error){if(selection===selectedTask)status.textContent=message(error,'Could not load the workspace.');return false}
  finally{setBusy(false)}
}
function renderMessages(container,data,task,before){
  container.replaceChildren();
  if(data.next_before!==null)container.append(button('Earlier messages',()=>showThread(task,data.next_before)));
  if(before!==null)container.append(button('Latest messages',()=>showThread(task)));
  if(data.messages.length===0){const empty=document.createElement('p');empty.textContent='No messages yet.';container.append(empty)}
  for(const item of data.messages){
    const article=document.createElement('article');
    const heading=document.createElement('h3');heading.textContent=item.role;
    const body=document.createElement('p');body.textContent=item.text+(item.truncated?'…':'');
    article.append(heading,body);container.append(article);
  }
}
// Each review is a server-issued one-shot receipt. Polls never recreate unchanged
// forms, and replies from a retired task or credential epoch cannot affect this UI.
function interactionPanel(base,current,onAnswered){
  const element=document.createElement('section');
  element.setAttribute('aria-label','Agent requests');
  const heading=document.createElement('h3');heading.textContent='Agent requests';
  const note=document.createElement('p');note.textContent='Only one-time permissions and task questions are supported. Login links are not opened here.';
  const list=document.createElement('div');element.append(heading,note,list);
  const rows=new Map();let order='';let available=true;
  function enable(row){for(const control of row.controls)control.disabled=!available||row.posting||!current()}
  function create(item){
    const card=document.createElement('article');const title=document.createElement('h4');title.textContent=item.title;card.append(title);
    const feedback=document.createElement('p');feedback.setAttribute('role','status');
    const row={card,controls:[],posting:false};
    function action(label,payload,review=false){
      const control=button(label,async()=>{
        if(!current()||row.posting||!available)return;
        let body;
        try{body=payload();if(new TextEncoder().encode(JSON.stringify(body)).length>65536)throw new Error('The response exceeds the 64 KiB limit.');}catch(error){feedback.textContent=message(error,'Check the form values.');return}
        if(review&&!confirm('Grant this agent the displayed one-time permission for this task?'))return;
        row.posting=true;enable(row);
        try{await postJson(base+'/interactions',body);if(current()){if(item.kind==='input')clearQuestionDraft(base,item.id);feedback.textContent='Response submitted.';await onAnswered()}}
        catch(error){if(current())feedback.textContent=message(error,'Response failed. Refresh before retrying.')}
        finally{row.posting=false;enable(row)}
      });
      row.controls.push(control);card.append(control);
    }
    if(item.kind==='permission'){
      const context=document.createElement('div');context.setAttribute('aria-label','Tool context');
      if(item.tool){
        const detail=document.createElement('p');detail.textContent=`Tool: ${item.tool.title||'Untitled'}${item.tool.kind?` (${item.tool.kind})`:''}`;context.append(detail);
        for(const diff of item.tool.diffs){
          const preview=document.createElement('details');const label=document.createElement('summary');label.textContent=`Recorded diff: ${diff.path}`;preview.append(label);
          for(const [name,value] of [['Before',diff.before],['After',diff.after]])if(value!==null){const heading=document.createElement('strong');heading.textContent=name;const content=document.createElement('pre');content.textContent=value;preview.append(heading,content)}
          if(diff.truncated){const note=document.createElement('p');note.textContent='This recorded diff was shortened. Review the complete change in the native workspace before granting permission.';preview.append(note)}
          context.append(preview);
        }
        for(const detail of item.tool.details||[]){
          const preview=document.createElement('details');const label=document.createElement('summary');label.textContent=detail.kind==='terminal'?'Recorded terminal output':'Recorded tool text';preview.append(label);
          const content=document.createElement('pre');content.textContent=detail.text;preview.append(content);
          if(detail.truncated){const note=document.createElement('p');note.textContent='Output was shortened; review the complete tool record in the native workspace.';preview.append(note)}
          if(detail.kind==='terminal'&&detail.exit_code!==null){const exit=document.createElement('p');exit.textContent=`Exit code: ${detail.exit_code}`;preview.append(exit)}
          context.append(preview);
        }
      }else{const missing=document.createElement('p');missing.textContent='No matching tool details are available. Review the agent request before granting access.';context.append(missing)}
      card.append(context);
      for(const choice of item.choices){
        if(choice.kind!=='allow_once'&&choice.kind!=='deny_once')continue;
        action(`${choice.kind==='allow_once'?'Allow once':'Deny once'}: ${choice.label}`,()=>({action:'permission',id:item.id,choice:choice.id}),choice.kind==='allow_once');
      }
      action('Cancel request',()=>({action:'permission',id:item.id,choice:null}));
    }else if(item.kind==='input'){
      const fields=[];
      const restored=readQuestionDraft(base,item);
      for(const field of item.fields){
        const label=document.createElement('label');label.textContent=field.label+(field.required?' (required)':' (optional)');
        const kind=field.kind;
        const control=document.createElement(kind.kind==='text'?'textarea':kind.kind==='number'?'input':'select');
        control.setAttribute('aria-label',field.label);control.autocomplete='off';
        if(kind.kind==='text'){control.rows=2;control.maxLength=65536}
        else if(kind.kind==='number'){control.type='number';control.step=kind.integer?'1':'any';if(kind.minimum!==null)control.min=String(kind.minimum);if(kind.maximum!==null)control.max=String(kind.maximum)}
        else{
          if(kind.kind==='multi_choice')control.multiple=true;
          else{const blank=document.createElement('option');blank.value='';blank.textContent='Choose a value';control.append(blank)}
          const options=kind.kind==='boolean'?[{value:'true',label:'Yes'},{value:'false',label:'No'}]:kind.options;
          for(const [index,option] of options.entries()){const entry=document.createElement('option');entry.value=kind.kind==='boolean'?option.value:String(index);entry.textContent=option.label;control.append(entry)}
        }
        if(restored&&Object.hasOwn(restored,field.id)){
          const saved=restored[field.id];
          if(kind.kind==='multi_choice'&&Array.isArray(saved))for(const option of control.options)option.selected=saved.includes(option.value);
          else if(typeof saved==='string'&&kind.kind!=='multi_choice')control.value=saved;
        }
        label.append(control);card.append(label);row.controls.push(control);fields.push({field,control});
      }
      function persist(){
        if(!current())return;
        const values=Object.create(null);
        for(const {field,control} of fields)values[field.id]=field.kind.kind==='multi_choice'?Array.from(control.selectedOptions,option=>option.value):control.value;
        if(!saveQuestionDraft(base,item,values))feedback.textContent='Could not save this question draft in the browser. Keep this page open until you submit.';
      }
      for(const {control} of fields){control.addEventListener('input',persist);control.addEventListener('change',persist)}
      const draftNote=document.createElement('small');draftNote.textContent='Unsubmitted answers are saved in this browser profile for up to 24 hours. They are not sent to the agent until you submit.';card.append(draftNote);
      const clear=button('Clear saved answers',()=>{if(!current())return;clearQuestionDraft(base,item.id);for(const {control} of fields){if(control.multiple)for(const option of control.options)option.selected=false;else control.value=''}feedback.textContent='Saved answers cleared.'});row.controls.push(clear);card.append(clear);
      action('Submit answers',()=>{
        // A null-prototype object preserves literal provider field IDs such as __proto__.
        const values=Object.create(null);
        for(const {field,control} of fields){
          const kind=field.kind.kind;
          if(kind==='multi_choice'){const selected=Array.from(control.options).filter(option=>option.selected).map(option=>field.kind.options[Number(option.value)].value);if(selected.length||field.required)values[field.id]=selected}
          else if(control.value!==''||field.required){
            if(kind==='boolean'){if(!control.value)throw new Error('Choose Yes or No for required fields.');values[field.id]=control.value==='true'}
            else if(kind==='number'){if(!control.value.trim()||!Number.isFinite(Number(control.value)))throw new Error('Enter a finite number.');values[field.id]=Number(control.value)}
            else if(kind==='choice'){if(control.value==='')throw new Error('Choose an offered value for required fields.');values[field.id]=field.kind.options[Number(control.value)].value}
            else values[field.id]=control.value;
          }
        }
        return {action:'input',id:item.id,response:{action:'accept',values}};
      });
      action('Decline question',()=>({action:'input',id:item.id,response:{action:'decline'}}));
      action('Cancel request',()=>({action:'input',id:item.id,response:{action:'cancel'}}));
    }
    card.append(feedback);return row;
  }
  return {element,
    update(data){
      available=true;
      const items=data.items||[];const ids=new Set(items.map(item=>item.id));
      for(const id of rows.keys())if(!ids.has(id)){clearQuestionDraft(base,id);rows.delete(id)}
      for(const item of items)if(!rows.has(item.id))rows.set(item.id,create(item));
      const next=JSON.stringify(items.map(item=>item.id));
      if(next!==order){list.replaceChildren(...items.map(item=>rows.get(item.id).card));order=next}
      for(const row of rows.values())enable(row);
      note.textContent=data.more?'More requests are queued. Respond to these to review the next requests.':items.length?'Agent-provided requests. Recorded tool context appears when available. Deny anything you cannot assess. Answers go to the configured agent.':'No pending requests. Connection sign-in and URL requests remain unsupported.';
    },
    unavailable(){available=false;for(const row of rows.values())enable(row);note.textContent='Requests could not be refreshed. Values are retained; refresh status before answering.'}
  };
}

async function showThread(task,before=null){
  if(!discardUnsaved())return;
  const selection=retireSelection();
  const base=`/api/tasks/${encodeURIComponent(task.id)}`;
  const path=base+'/thread'+(before===null?'':`?before=${before}`);
  const current=()=>selection===selectedTask;
  thread.replaceChildren();
  const loading=document.createElement('p');loading.textContent='Loading messages…';thread.append(loading);
  try{
    const [data,unsent,initialRun,initialInteractions]=await Promise.all([getJson(path),getJson(base+'/draft'),getJson(base+'/run'),getJson(base+'/interactions')]);
    if(!current())return;
    const title=document.createElement('h2');title.textContent=data.title;thread.replaceChildren(title);
    const state=document.createElement('small');state.textContent=`State: ${data.state}`;thread.append(state);
    const runStatus=document.createElement('p');runStatus.setAttribute('role','status');thread.append(runStatus);
    const pending=document.createElement('article');const heading=document.createElement('h3');heading.textContent='Task draft';pending.append(heading);
    let running=false;
    let posting=false;
    let route=initialRun.route;
    let remote=initialRun.remote;
    let runButton=null;
    let saveButton=null;
    const stopButton=button('Stop run',async()=>{
      if(!current()||posting)return;
      posting=true;updateButtons();
      try{const result=await postJson(base+'/stop',{});if(current())showRun(result)}
      catch(error){if(current())status.textContent=message(error,'Could not stop the run.')}
      finally{posting=false;updateButtons();if(current())schedulePoll()}
    });
    function updateButtons(){stopButton.disabled=!running||posting;if(runButton)runButton.disabled=running||posting;if(saveButton)saveButton.disabled=posting}
    function showRun(run){if(Object.hasOwn(run,'route'))route=run.route;if(Object.hasOwn(run,'remote'))remote=run.remote;running=run.state==='running'||run.state==='stopping';runStatus.textContent=`Web run: ${run.state}${run.error?` (${run.error})`:''}. ${route?`Direct model: ${route.provider_id} / ${route.model_id}`:remote?'ACP agent on SSH workspace':'Local ACP agent'}${remote?` · SSH: ${remote.host}`:''}`;updateButtons()}
    if(unsent.truncated){const clipped=document.createElement('p');clipped.textContent=unsent.text+'…';pending.append(clipped);const note=document.createElement('small');note.textContent='This draft is too large to edit or run in the local browser.';pending.append(note)}
    else{
      const editor=document.createElement('textarea');editor.rows=5;editor.value=unsent.text;editor.setAttribute('aria-label','Task draft');
      activeDraftEditor=editor;activeDraftSaved=unsent.text;
      saveButton=button('Save draft',async()=>{
        if(!current()||posting)return;
        const text=editor.value;
        if(new TextEncoder().encode(text).length>16384){status.textContent='The task draft is too large for the local server.';return}
        posting=true;updateButtons();
        try{await postJson(base+'/draft',{text});if(current()){activeDraftSaved=text;status.textContent='Draft saved. Nothing was sent.'}}
        catch(error){if(current())status.textContent=message(error,'Could not save the draft.')}
        finally{posting=false;updateButtons()}
      });
      runButton=button('Run draft',async()=>{
        if(!current()||posting||running)return;
        const text=editor.value;
        if(!text.trim()||new TextEncoder().encode(text).length>16384||text.includes('\0')){status.textContent='Enter a non-empty prompt of at most 16 KiB without NUL characters.';return}
        const review=route?`Run this prompt with the reviewed direct model ${route.provider_id} / ${route.model_id}? This sends the prompt and selected history to its configured provider. It does not grant agent tools. Provider charges may apply.${remote?` This task belongs to the SSH workspace ${remote.host}, but the direct model does not run tools there.`:''} The draft is retained for review and retry.`:remote?`Run this prompt with the configured agent on SSH workspace ${remote.host}? The agent can use its existing remote permissions. One-time permission requests and task questions require explicit replies here. Sign-in links are not supported. The draft is retained for review and retry.`:'Run this prompt with the configured local agent in this task folder? The agent can use its existing local permissions. One-time permission requests and task questions require explicit replies in this page. Sign-in links are not supported. The draft is retained for review and retry.';
        if(!confirm(review))return;
        const expected_draft=activeDraftSaved;
        const expected_route=route?.stamp||null;
        const expected_remote=remote?.stamp||null;
        posting=true;updateButtons();
        try{
          const result=await postJson(base+'/run',{text,expected_draft,expected_route,expected_remote});
          if(current()){activeDraftSaved=text;showRun(result);status.textContent='Run started. Draft retained. Stop cancels this task run.';schedulePoll()}
        }catch(error){if(current())status.textContent=message(error,'Could not start the run. Reload status before retrying.')}
        finally{posting=false;updateButtons()}
      });
      pending.append(editor,saveButton,runButton);
    }
    pending.append(stopButton);thread.append(pending);
    const reviews=interactionPanel(base,current,refresh);thread.append(reviews.element);reviews.update(initialInteractions);
    const messages=document.createElement('div');thread.append(messages);renderMessages(messages,data,task,before);
    function schedulePoll(){clearTimeout(pollTimer);if(current())pollTimer=setTimeout(refresh,1000)}
    let refreshSequence=0;
    async function refresh(){
      if(!current())return;
      const sequence=++refreshSequence;
      try{
        const [run,latest,interactions]=await Promise.all([getJson(base+'/run'),getJson(path),getJson(base+'/interactions')]);
        if(!current()||sequence!==refreshSequence)return;
        reviews.update(interactions);showRun(run);state.textContent=`State: ${latest.state}`;renderMessages(messages,latest,task,before);
        if(running)schedulePoll();
      }catch(error){if(current()&&sequence===refreshSequence){reviews.unavailable();runStatus.textContent='Live status unavailable. Reload to reconnect; a run may still be active.';status.textContent=message(error,'Could not refresh status.')}}
    }
    thread.append(button('Refresh status and messages',refresh));
    showRun(initialRun);if(running)schedulePoll();
  }catch(error){if(current()){loading.textContent=message(error,'Could not load the task.');thread.replaceChildren(loading)}}
}
// Changing credentials retires in-flight reads and polling. Credentials are never stored.
token.addEventListener('input',()=>{const editor=activeDraftEditor;retireSelection();activeDraftEditor=editor;if(editor)editor.readOnly=true;status.textContent='Credentials changed. Copy any unsaved draft, then load the workspace again.'});
load.addEventListener('click',()=>loadWorkspace());
addWorkspace.addEventListener('click',async()=>{
  if(!discardUnsaved())return;
  if(!root.value.trim()){status.textContent='Enter an existing absolute folder path.';return}
  setBusy(true);
  try{const added=await postJson('/api/workspaces',{root:root.value.trim()});if(await loadWorkspace()){if(!Array.from(project.options).some(option=>option.value===added.id)){const option=document.createElement('option');option.value=added.id;option.textContent=added.name;project.append(option)}project.value=added.id;status.textContent='Local folder added as a project.'}}
  catch(error){status.textContent=message(error,'Could not add the folder.')}
  finally{setBusy(false)}
});
createTask.addEventListener('click',async()=>{
  if(!discardUnsaved())return;
  if(!project.value||!profile.value||!titleInput.value.trim()){status.textContent='Choose a project and agent profile, then enter a task title.';return}
  if(new TextEncoder().encode(titleInput.value.trim()).length>400){status.textContent='The task title is too long.';return}
  if(new TextEncoder().encode(JSON.stringify({title:titleInput.value.trim(),agent_id:profile.value,draft:draft.value})).length>60000){status.textContent='The task draft is too large for the local server.';return}
  if(new TextEncoder().encode(draft.value).length>16384){status.textContent='The task draft is too large for the local server.';return}
  setBusy(true);
  try{const created=await postJson(`/api/projects/${encodeURIComponent(project.value)}/tasks`,{title:titleInput.value.trim(),agent_id:profile.value,draft:draft.value});titleInput.value='';draft.value='';if(await loadWorkspace(created.id))status.textContent='Unsent task and draft created.'}
  catch(error){status.textContent=message(error,'Could not create the task.')}
  finally{setBusy(false)}
});
