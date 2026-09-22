#!/usr/bin/env python3
"""PR Fix using real X11 and the existing gh process owner, on an owned fixture repo.
No GitHub network, credentials, SQL writes or provider mutations are permitted.
"""
import argparse
import json
import os
import subprocess
import time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, click, fresh_probe
from native_navigation_smoke import selection
from native_project_import_smoke import task_events
from native_model_draft_smoke import preference, close


def fixture(s):
    subprocess.run(['git','init','-q',str(s.project)],check=True)
    subprocess.run(['git','-C',str(s.project),'remote','add','origin','https://github.com/owner/project.git'],check=True)
    folder=s.output/'bin'; folder.mkdir()
    state=s.output/'provider.json'
    state.write_text(json.dumps({'head':'a'*40,'body':'Check this line','delay':0}))
    log=s.output/'requests.jsonl'
    program='''#!/usr/bin/python3
import json, sys, time
from pathlib import Path
state=json.loads(Path(STATE).read_text())
args=sys.argv[1:]; method=args[args.index('--method')+1]; endpoint=args[-1]
request=json.load(sys.stdin) if '--input' in args else None
with Path(LOG).open('a') as stream: stream.write(json.dumps({'method':method,'endpoint':endpoint,'request':request})+'\\n')
assert method=='GET' or method=='POST' and endpoint=='graphql' and request['query'].lstrip().startswith('query')
pr={'number':7,'title':'Fixture PR','state':'open','draft':False,'user':{'login':'author'},'body':'Review fixture','head':{'sha':'a'*40,'ref':'feature'},'base':{'ref':'main'}}
if endpoint=='graphql':
 deadline=time.monotonic()+20
 while json.loads(Path(STATE).read_text()).get('hold',False):
  if time.monotonic()>=deadline: raise RuntimeError('Test did not release the review response barrier')
  time.sleep(.025)
 time.sleep(state['delay'])
 comment={'id':'C1','url':'https://github.com/owner/project/pull/7#discussion_r1','body':state['body'],'diffHunk':'@@ -1 +1 @@\\n-old\\n+new','createdAt':'2026-01-01T00:00:00Z','updatedAt':'2026-01-02T00:00:00Z','author':{'login':'reviewer'},'commit':{'oid':'a'*40}}
 thread={'id':'T1','path':'document.txt','line':1,'startLine':None,'originalLine':1,'originalStartLine':None,'diffSide':'RIGHT','startDiffSide':None,'isResolved':False,'isOutdated':False,'comments':{'pageInfo':{'hasNextPage':False},'nodes':[comment]}}
 value={'data':{'repository':{'pullRequest':{'number':7,'state':'OPEN','headRefOid':state['head'],'reviewThreads':{'pageInfo':{'hasNextPage':False},'nodes':[thread]}}}}}
elif endpoint.startswith('search/issues'): value={'incomplete_results':False,'items':[pr]}
elif endpoint.endswith('/pulls/7'): value=pr
elif '/check-runs' in endpoint: value={'check_runs':[]}
elif '/status?' in endpoint: value={'statuses':[]}
else: value=[]
print(json.dumps(value))
'''.replace('STATE',repr(str(state))).replace('LOG',repr(str(log)))
    exe=folder/'gh';exe.write_text(program);exe.chmod(0o700)
    os.environ['PATH']=str(folder)+os.pathsep+os.environ['PATH']
    return state,log


def set_provider(state, *, head='a'*40, body='Check this line', hold=False):
    # Atomic fixture updates make response release deterministic.
    pending = state.with_suffix('.next')
    pending.write_text(json.dumps({'head': head, 'body': body, 'delay': 0, 'hold': hold}))
    pending.replace(state)


def query_count(log):
    if not log.exists():
        return 0
    return sum(json.loads(line)['endpoint'] == 'graphql'
               for line in log.read_text().splitlines())


def run(s):
    state,log=fixture(s)
    s.launch(); source=selection(s)
    fill(s,'composer-input','Preserve this request')
    wait_until(lambda: (preference(s,'task-draft:'+source) or {}).get('text')=='Preserve this request','draft persistence')
    before=task_events(s,source)
    click(s,'pull-requests-navigation');click(s,'pr-discover')
    wait_until(lambda:s.control_bounds('pr-remote',0),'repository discovery')
    s.click_control('pr-remote',slot=0)
    wait_until(lambda:s.control_bounds('pr-list-row',0),'PR list')
    s.click_control('pr-list-row',slot=0)
    click(s,'pr-fix-collect')
    wait_until(lambda:s.control_bounds('pr-fix-instruction'),'reviewed PR context')
    fill(s,'pr-fix-instruction','Fix only this reviewed issue')
    s.desktop.screenshot('pr-fix-reviewed',window_only=True)
    assert task_events(s,source)==before
    assert preference(s,'task-draft:'+source)['text']=='Preserve this request'
    s.checks.append('read-only-collection-preserves-task-history-and-existing-draft')
    state.write_text(json.dumps({'head':'b'*40,'body':'Check this line','delay':0}))
    click(s,'pr-fix-add')
    wait_until(lambda:s.control_bounds('pr-fix-error'),'stale PR head rejected')
    assert preference(s,'task-draft:'+source)['text']=='Preserve this request'
    assert task_events(s,source)==before
    s.checks.append('stale-head-recheck-rejects-context-without-draft-or-provider-mutation')
    state.write_text(json.dumps({'head':'a'*40,'body':'Check this line','delay':0}))
    fresh_probe(s, 'pr-fix-instruction', lambda: click(s, 'pr-fix-collect'))
    fill(s,'pr-fix-instruction','Fix only this reviewed issue')
    set_provider(state, body='Review changed after collection')
    click(s,'pr-fix-add')
    wait_until(lambda:s.control_bounds('pr-fix-error'),'changed comments rejected')
    assert preference(s,'task-draft:'+source)['text']=='Preserve this request'
    assert task_events(s,source)==before
    s.checks.append('changed-review-comments-are-rejected-without-inserting-stale-context')

    set_provider(state, hold=True)
    requests_before=query_count(log)
    click(s,'pr-fix-add')
    wait_until(lambda:query_count(log)==requests_before+1,'recheck response held by fixture')
    fill(s,'pr-fix-instruction','Edited while rechecking')
    s.click_control('pr-fix-instruction')
    assert s.desktop.copy_input()=='Edited while rechecking'
    set_provider(state)
    wait_until(lambda:s.control_bounds('pr-fix-error'),'in-flight instruction edit rejected')
    assert preference(s,'task-draft:'+source)['text']=='Preserve this request'
    assert task_events(s,source)==before
    s.click_control('pr-fix-instruction')
    assert s.desktop.copy_input()=='Edited while rechecking'
    s.checks.append('instruction-edits-during-recheck-retain-user-work-and-block-old-draft-insertion')

    set_provider(state, hold=True)
    requests_before=query_count(log)
    click(s,'pr-fix-add')
    wait_until(lambda:query_count(log)==requests_before+1,'cancel test response held')
    # Geometry lookup retains historical rows; it cannot prove disappearance.
    # Require a new actual render of the no-review state after Cancel instead.
    fresh_probe(s, 'pr-fix-idle', lambda: click(s, 'pr-fix-discard'))
    set_provider(state)
    assert preference(s,'task-draft:'+source)['text']=='Preserve this request'
    assert task_events(s,source)==before
    s.checks.append('cancelled-recheck-cannot-insert-context-from-a-retired-review')

    fresh_probe(s, 'pr-fix-instruction', lambda: click(s, 'pr-fix-collect'))
    fill(s,'pr-fix-instruction','Fix only this reviewed issue')
    click(s,'pr-fix-add')
    wait_until(lambda:'PR Fix for owner/project #7' in (preference(s,'task-draft:'+source) or {}).get('text',''),'unsent PR Fix appended')
    draft=preference(s,'task-draft:'+source)['text']
    assert draft.startswith('Preserve this request\n\n') and 'Fix only this reviewed issue' in draft
    assert all(value in draft for value in ['document.txt','discussion_r1','UNTRUSTED','a'*40])
    assert selection(s)==source and task_events(s,source)==before
    s.desktop.screenshot('pr-fix-unsent-draft',window_only=True)
    s.checks.append('editable-instruction-and-identities-enter-only-the-correct-unsent-composer')
    close(s);s.launch(preserve_selection=True)
    assert preference(s,'task-draft:'+source)['text']==draft and task_events(s,source)==before
    requests=[json.loads(line) for line in log.read_text().splitlines()]
    assert all(r['method']=='GET' or r['endpoint']=='graphql' and r['request']['query'].lstrip().startswith('query') for r in requests)
    assert s.document.read_text()=='original text\n'
    s.checks.append('draft-survives-restart-without-send-checkout-or-GitHub-write')
    close(s)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--fixture',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    s=Scenario(parser.parse_args());result={'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb / owned read-only gh fixture'}
    try: run(s);result['status']='passed'
    except BaseException as error:
        result['error']=str(error)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:
        s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))

if __name__=='__main__':main()
