'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const fs=require('node:fs');
const source=fs.readFileSync('crates/synara-server/src/app.js','utf8');
class Element {
  constructor(tag='div'){this.tag=tag;this.value='';this.textContent='';this.children=[];this.listeners={};this.disabled=false;this.attributes={}}
  append(...children){this.children.push(...children)}
  replaceChildren(...children){this.children=[...children]}
  setAttribute(name,value){this.attributes[name]=value}
  addEventListener(name,fn){this.listeners[name]=fn}
  async click(){if(!this.disabled)await this.listeners.click?.()}
  get options(){return this.children}
}
function fixture(){
  const nodes=new Map();
  const requests=[];const timers=new Map();let timerId=0;
  const pages={a:{title:'Task A',draft:'Original'},b:{title:'Task B',draft:'Other'}};
  let postGate=null;let confirmValue=true;
  const ctx=vm.createContext({console,TextEncoder,Error,document:{querySelector(id){if(!nodes.has(id))nodes.set(id,new Element());return nodes.get(id)},createElement(tag){return new Element(tag)}},
    confirm(){return confirmValue},setTimeout(fn){timers.set(++timerId,fn);return timerId},clearTimeout(id){timers.delete(id)},
    async fetch(path,options){
      requests.push({path,options});const id=path.split('/')[3];
      if(options.method==='POST'){
        if(postGate)await postGate;
        if(path.endsWith('/run'))return {ok:true,status:202,json:async()=>({state:'running',error:null})};
        if(path.endsWith('/stop'))return {ok:true,status:200,json:async()=>({state:'stopping',error:null})};
        return {ok:true,status:200,json:async()=>({})};
      }
      const page=pages[id]||pages.a;
      const data=path.endsWith('/interactions')?{items:page.interactions||[],more:false}:path.endsWith('/draft')?{text:page.draft,truncated:false}:path.endsWith('/run')?{state:'idle',error:null}:{title:page.title,state:'completed',messages:[{role:'assistant',text:'literal <script>not executable</script>',truncated:false}],next_before:null};
      return {ok:true,status:200,json:async()=>data};
    }});
  vm.runInContext(source,ctx);
  const evaluate=text=>vm.runInContext(text,ctx);
  const all=(node)=>[node,...node.children.flatMap(all)];
  return {nodes,ctx,requests,timers,evaluate,show:id=>evaluate(`showThread({id:'${id}'})`),
    setInteractions(id,items){pages[id].interactions=items},
    field:label=>all(nodes.get('#thread')).find(node=>node.attributes?.['aria-label']===label),
    editor:()=>all(nodes.get('#thread')).find(node=>node.tag==='textarea'),
    button:name=>all(nodes.get('#thread')).find(node=>node.textContent===name),
    setGate(gate){postGate=gate},denyDiscard(){confirmValue=false}};
}
test('loading task never executes an agent and renders prompt text literally',async()=>{
  const f=fixture();await f.show('a');
  assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);
  assert.equal(f.editor().value,'Original');assert.equal(f.button('Run draft').disabled,false);
});
test('run uses reviewed snapshot and does not mark later typing as saved',async()=>{
  const f=fixture();await f.show('a');f.editor().value='Send this';
  let resolve;f.setGate(new Promise(r=>resolve=r));
  const pending=f.button('Run draft').click();await Promise.resolve();
  f.editor().value='New unsaved text';resolve();await pending;
  const body=JSON.parse(f.requests.find(r=>r.options.method==='POST').options.body);
  assert.deepEqual(body,{text:'Send this',expected_draft:'Original'});
  assert.equal(f.evaluate('activeDraftSaved'),'Send this');
  assert.equal(f.editor().value,'New unsaved text');
  assert.equal(f.button('Run draft').disabled,true);
  assert.equal(f.button('Stop run').disabled,false);
  assert.equal(f.timers.size,1);
  // Poll only replaces messages and status, not the editor or its selection owner.
  const editor=f.editor();await [...f.timers.values()][0]();
  assert.equal(f.editor(),editor);assert.equal(editor.value,'New unsaved text');
});
test('save response records the submitted value rather than newer edits',async()=>{
  const f=fixture();await f.show('a');f.editor().value='Save this';
  let resolve;f.setGate(new Promise(r=>resolve=r));
  const pending=f.button('Save draft').click();await Promise.resolve();
  f.editor().value='Keep unsaved';resolve();await pending;
  assert.equal(f.evaluate('activeDraftSaved'),'Save this');
  f.denyDiscard();await f.show('b');assert.equal(f.editor().value,'Keep unsaved');
});
test('late post results cannot modify another task or schedule its polling',async()=>{
  const f=fixture();await f.show('a');
  let resolve;f.setGate(new Promise(r=>resolve=r));const pending=f.button('Run draft').click();
  await Promise.resolve();await f.show('b');resolve();await pending;
  assert.equal(f.editor().value,'Other');assert.equal(f.evaluate('activeDraftSaved'),'Other');
  assert.equal(f.timers.size,0);assert.equal(f.button('Run draft').disabled,false);
});
test('credential changes retire actions but preserve unsaved text for copying',async()=>{
  const f=fixture();await f.show('a');f.editor().value='Keep me';
  const run=f.button('Run draft');f.nodes.get('#token').listeners.input();await run.click();
  assert.equal(f.editor().value,'Keep me');assert.equal(f.editor().readOnly,true);
  assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);
  assert.equal(f.timers.size,0);
});

test('web questions preserve typed values across polling and submit literal field IDs',async()=>{
  const f=fixture();f.setInteractions('a',[{id:'review-1',kind:'input',title:'Literal <script>request</script>',fields:[{id:'__proto__',label:'Reply',required:true,kind:{kind:'text',max_length:1000}}]}]);
  await f.show('a');const field=f.field('Reply');field.value='Answer stays typed';
  await f.button('Refresh status and messages').click();assert.equal(f.field('Reply'),field);assert.equal(field.value,'Answer stays typed');
  await f.button('Submit answers').click();
  const post=f.requests.find(r=>r.options.method==='POST');assert.equal(post.path,'/api/tasks/a/interactions');
  assert.deepEqual(JSON.parse(post.options.body),{action:'input',id:'review-1',response:{action:'accept',values:JSON.parse('{"__proto__":"Answer stays typed"}')}});
  assert.equal(f.editor().value,'Original');
});
test('web permissions require a current explicit once-only action and confirmation',async()=>{
  const f=fixture();f.setInteractions('a',[{id:'receipt',kind:'permission',title:'Write file',choices:[{id:'once',label:'This tool',kind:'allow_once'},{id:'always',label:'Forever',kind:'allow_always'}]}]);
  await f.show('a');assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);assert.equal(f.button('Allow once: Forever'),undefined);
  f.denyDiscard();await f.button('Allow once: This tool').click();assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);
  await f.button('Cancel request').click();assert.deepEqual(JSON.parse(f.requests.find(r=>r.options.method==='POST').options.body),{action:'permission',id:'receipt',choice:null});
});
test('old request controls cannot answer after navigation or credential changes',async()=>{
  const f=fixture();const review={id:'same-provider-id',kind:'permission',title:'Write',choices:[{id:'once',label:'Once',kind:'allow_once'}]};
  f.setInteractions('a',[review]);await f.show('a');const stale=f.button('Allow once: Once');await f.show('b');await stale.click();
  assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);
  f.setInteractions('b',[review]);await f.show('b');const retired=f.button('Allow once: Once');f.nodes.get('#token').listeners.input();await retired.click();
  assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);
});
test('a late question reply cannot change a newer task draft or review',async()=>{
  const f=fixture();f.setInteractions('a',[{id:'question-a',kind:'input',title:'Question',fields:[]}]);await f.show('a');
  let resolve;f.setGate(new Promise(r=>resolve=r));const pending=f.button('Decline question').click();await Promise.resolve();await f.show('b');resolve();await pending;
  assert.equal(f.editor().value,'Other');assert.equal(f.button('Decline question'),undefined);assert.equal(f.timers.size,0);
});

// Option indices distinguish the blank placeholder from legitimate empty values.
test('question choices preserve empty values and multiselect identity',async()=>{
  const f=fixture();const options=[{value:'',label:'Empty'},{value:'one',label:'First'}];
  f.setInteractions('a',[{id:'choices',kind:'input',title:'Choose',fields:[
    {id:'single',label:'Single',required:false,kind:{kind:'choice',options}},
    {id:'multiple',label:'Multiple',required:true,kind:{kind:'multi_choice',options}},
    {id:'truth',label:'Truth',required:true,kind:{kind:'boolean'}}
  ]}]);
  await f.show('a');f.field('Single').value='0';f.field('Multiple').options[1].selected=true;f.field('Truth').value='false';
  await f.button('Submit answers').click();
  assert.deepEqual(JSON.parse(f.requests.find(r=>r.options.method==='POST').options.body).response.values,{single:'',multiple:['one'],truth:false});
});
test('oversized answers stay editable and do not send a request',async()=>{
  const f=fixture();f.setInteractions('a',[{id:'q',kind:'input',title:'Question',fields:[{id:'text',label:'Text',required:true,kind:{kind:'text'}}]}]);
  await f.show('a');const field=f.field('Text');field.value='界'.repeat(24000);await f.button('Submit answers').click();
  assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);assert.equal(field.disabled,false);assert.equal(field.value.length,24000);
});

test('required single choice never defaults to the first agent option',async()=>{
  const f=fixture();f.setInteractions('a',[{id:'q',kind:'input',title:'Question',fields:[{id:'choice',label:'Choice',required:true,kind:{kind:'choice',options:[{value:'grant',label:'Grant'}]}}]}]);
  await f.show('a');await f.button('Submit answers').click();assert.equal(f.requests.filter(r=>r.options.method==='POST').length,0);
});
