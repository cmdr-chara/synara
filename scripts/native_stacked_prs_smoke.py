#!/usr/bin/env python3
"""Real GPUI stack review against an owned gh process fixture, never GitHub.
The fixture deliberately supports only the exact scoped PR API surface below.
"""
import argparse
import json
import os
import subprocess
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import click, fresh_probe
from native_model_draft_smoke import close


def fixture(s):
    subprocess.run(['git','init','-q',str(s.project)],check=True)
    subprocess.run(['git','-C',str(s.project),'remote','add','origin','https://github.com/owner/project.git'],check=True)
    folder=s.output/'bin';folder.mkdir()
    state=s.output/'provider.json';log=s.output/'requests.jsonl'
    state.write_text(json.dumps({'changed':False,'merged':[],'retargeted':[],'mode':'ready'}))
    program=r'''#!/usr/bin/python3
import json,sys,time
from pathlib import Path
state_path=Path(STATE);state=json.loads(state_path.read_text())
args=sys.argv[1:];method=args[args.index('--method')+1];endpoint=args[-1]
request=json.load(sys.stdin) if '--input' in args else None
with Path(LOG).open('a') as f:f.write(json.dumps({'method':method,'endpoint':endpoint,'request':request})+'\n')
def pr(n):
    heads={1:'one',2:'two',3:'sibling'}
    bases={1:'main',2:'one',3:'one'}
    return {'number':n,'title':'Stack fixture '+str(n),'state':'open','draft':False,'merged':False,'user':{'login':'author'},'body':'Owned stack fixture',
      'mergeable':True,'mergeable_state':'clean',
      'head':{'sha':('b'*40 if state['changed'] and n==2 else format(n,'040x')),'ref':heads[n],'repo':{'full_name':'owner/project'}},
      'base':{'sha':'a'*40,'ref':('main' if n in state['retargeted'] else bases[n]),'repo':{'full_name':'owner/project'}}}
if method=='GET':
    if endpoint.startswith('search/issues'):value={'incomplete_results':False,'items':[pr(2)]}
    elif endpoint.startswith('repos/owner/project/pulls?'):value=[pr(n) for n in [3,2,1] if n not in state['merged']]
    elif endpoint in ['repos/owner/project/pulls/'+str(n) for n in [1,2,3]]:value=pr(int(endpoint.rsplit('/',1)[-1]))
    elif '/check-runs?' in endpoint:
        value={'total_count':1,'check_runs':[{'name':'fixture','status':'completed','conclusion':'failure' if state['mode']=='failed' else 'success'}]}
    elif '/status?' in endpoint:value={'total_count':0,'state':'pending','statuses':[]}
    else:
        assert endpoint.endswith(('/files?per_page=100','/commits?per_page=100','/reviews?per_page=100','/timeline?per_page=100')),endpoint
        value=[]
elif method=='PATCH':
    assert endpoint=='repos/owner/project/pulls/2' and request=={'base':'main'},(endpoint,request)
    assert state['merged']==[1]
    state['retargeted'].append(2);state_path.write_text(json.dumps(state));value=pr(2)
    if state['mode']=='cancel':time.sleep(8)
elif method=='PUT':
    n=int(endpoint.split('/')[-2]);assert n in [1,2] and endpoint==f'repos/owner/project/pulls/{n}/merge'
    assert request=={'sha':format(n,'040x'),'merge_method':'merge'},request
    assert not state['changed'] and state['mode']!='failed'
    if n==2:assert state['merged']==[1] and state['retargeted']==[2]
    state['merged'].append(n);state_path.write_text(json.dumps(state));value={'merged':True,'sha':'f'*40}
else:raise AssertionError((method,endpoint))
print(json.dumps(value))
'''.replace('STATE',repr(str(state))).replace('LOG',repr(str(log)))
    program=program.replace("+'\\\\n'", "+'\\n'")
    exe=folder/'gh';exe.write_text(program);exe.chmod(0o700)
    os.environ['PATH']=str(folder)+os.pathsep+os.environ['PATH']
    return state,log


def read(path):return json.loads(path.read_text())
def change(path,**values):
    state=read(path);state.update(values);path.write_text(json.dumps(state))
def writes(log):return [v for v in (json.loads(x) for x in log.read_text().splitlines()) if v['method']!='GET']
def review(s):
    fresh_probe(s,'pr-stack-row',lambda:click(s,'pr-stack-load'))
    wait_until(lambda:s.control_bounds('pr-stack-row',2),'complete deterministic stack rows')
def prepare(s):
    fresh_probe(s,'pr-stack-confirmation',lambda:click(s,'pr-stack-prepare'))


def run(s):
    state,log=fixture(s)
    s.launch()
    before=s.document.read_bytes()
    click(s,'pull-requests-navigation');click(s,'pr-discover')
    wait_until(lambda:s.control_bounds('pr-remote',0),'repository discovery')
    s.click_control('pr-remote',slot=0)
    wait_until(lambda:s.control_bounds('pr-list-row',0),'PR list')
    s.click_control('pr-list-row',slot=0)
    review(s);prepare(s)
    s.desktop.screenshot('stack-reviewed-prefix',window_only=True)
    assert writes(log)==[]
    click(s,'pr-stack-dismiss');assert writes(log)==[]
    s.checks.append('deterministic-stack-current-position-review-and-cancel-with-zero-writes')
    prepare(s);change(state,changed=True)
    fresh_probe(s,'pr-error',lambda:click(s,'pr-stack-confirm'))
    assert writes(log)==[]
    s.checks.append('stale-descendant-head-rejects-entire-prefix-before-root-merge')
    change(state,changed=False,mode='failed');review(s)
    offset=Path(s.log.name).stat().st_size
    fresh_probe(s,'pr-error',lambda:click(s,'pr-stack-prepare'))
    assert 'pr-stack-confirmation' not in Path(s.log.name).read_bytes()[offset:].decode(errors='replace')
    assert writes(log)==[]
    s.checks.append('failed-checks-cannot-reach-confirmation-or-provider-write')
    change(state,mode='cancel');review(s);prepare(s);click(s,'pr-stack-confirm')
    wait_until(lambda:read(state)['retargeted']==[2],'child retarget request')
    fresh_probe(s,'pr-stack-progress',lambda:click(s,'pr-cancel'))
    assert read(state)['merged']==[1] and read(state)['retargeted']==[2]
    assert len(writes(log))==2
    s.desktop.screenshot('stack-cancelled-partial-progress',window_only=True)
    s.checks.append('stop-preserves-confirmed-root-merge-and-does-not-retry-ambiguous-retarget')
    # Reset only owned fake-provider state, never app database or live GitHub.
    change(state,merged=[],retargeted=[],mode='ready')
    review(s);prepare(s)
    fresh_probe(s,'pr-stack-progress',lambda:click(s,'pr-stack-confirm'))
    assert read(state)['merged']==[1,2]
    requests=writes(log)
    assert [(r['method'],r['endpoint']) for r in requests[-3:]]==[
        ('PUT','repos/owner/project/pulls/1/merge'),('PATCH','repos/owner/project/pulls/2'),('PUT','repos/owner/project/pulls/2/merge')]
    assert all('/pulls/3' not in r['endpoint'] for r in requests)
    assert s.document.read_bytes()==before
    s.desktop.screenshot('stack-completed-prefix',window_only=True)
    s.checks.append('exact-reviewed-prefix-retargets-root-first-with-pinned-heads-and-no-sibling-or-local-file-change')
    count=len(requests);close(s);s.launch(preserve_selection=True)
    assert len(writes(log))==count and s.document.read_bytes()==before
    s.checks.append('restart-does-not-replay-any-reviewed-or-ambiguous-stack-write')
    close(s)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--fixture',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    s=Scenario(parser.parse_args());result={'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb / owned GitHub CLI fixture'}
    try:run(s);result['status']='passed'
    except BaseException as error:
        result['error']=str(error)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:
        s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))

if __name__=='__main__':main()
