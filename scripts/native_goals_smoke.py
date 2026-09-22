#!/usr/bin/env python3
"""Real bounded goal pursuit and restart/user-input/approval interruption paths."""
import argparse,json,time
from pathlib import Path
from native_smoke import Scenario,wait_until
from native_navigation_smoke import selection
from native_model_draft_smoke import preference,close
from native_integrations_smoke import fill,click

def count(s):
    return sum(e['type']=='prompt_started' for e in s.events())

def save(s,task,text):
    key='task-goal:'+task
    fill(s,'goal-input',text)
    click(s,'goal-save')
    wait_until(lambda: (preference(s,key) or {}).get('objective')==text,'goal saved')
    assert preference(s,key)['status']=='paused'

def run(s):
    s.launch()
    before=len(s.events())
    fill(s,'composer-input','hello')
    s.click_control('composer-submit',enabled=True)
    s.finished(before)
    task=selection(s);key='task-goal:'+task
    click(s,'goal-open')
    save(s,task,'fixture-goal-budget')
    initial=count(s)
    s.click_control('goal-resume',enabled=True)
    s.click_control('composer-input')
    request=s.desktop.copy_input()
    assert task in request and 'at most two' in request and 'current task' in request
    assert count(s)==initial
    click(s,'goal-pause')
    wait_until(lambda: 'Paused by you' in preference(s,key)['note'],'explicit pause')
    close(s);s.launch(preserve_selection=True)
    time.sleep(1)
    assert count(s)==initial and preference(s,key)['status']=='paused'
    click(s,'goal-open')
    fill(s,'composer-input','')
    s.click_control('goal-resume',enabled=True);click(s,'composer-submit')
    wait_until(lambda: count(s)>initial,'explicit Send keeps the goal lease and starts the turn')
    wait_until(lambda: preference(s,key)['status']=='blocked','bounded goal pursuit exhausted',30)
    assert 'budget' in preference(s,key)['note'] and count(s)==initial+3
    assert preference(s,key)['elapsed_ms']>0
    s.desktop.screenshot('goal-budget-exhausted',window_only=True)
    s.checks.append('explicit-inert-resume-and-restart-with-exactly-two-automatic-followups')
    save(s,task,'fixture-goal-budget')
    initial=count(s)
    s.click_control('goal-resume',enabled=True);click(s,'composer-submit')
    wait_until(lambda: 'Follow-up preview' in preference(s,key)['note'],'visible countdown')
    fill(s,'composer-input','My manual message has priority')
    wait_until(lambda: 'User input' in preference(s,key)['note'],'new user input pauses continuation')
    time.sleep(6)
    assert count(s)==initial+1
    assert preference(s,'task-draft:'+task)['text']=='My manual message has priority'
    s.checks.append('user-draft-priority-cancels-countdown-without-overwriting-or-sending-it')
    fill(s,'composer-input','')
    save(s,task,'fixture-goal-question');initial=count(s)
    s.click_control('goal-resume',enabled=True);click(s,'composer-submit')
    wait_until(lambda: preference(s,key)['status']=='blocked','question blocks pursuit')
    assert 'question' in preference(s,key)['note'] and count(s)==initial+1
    save(s,task,'fixture-goal-permission');initial=count(s)
    s.click_control('goal-resume',enabled=True);click(s,'composer-submit')
    wait_until(lambda:s.task()['state']=='waiting','explicit approval blocks goal')
    wait_until(lambda:preference(s,key)['status']=='blocked','approval disarms continuation')
    click(s,'permission-deny-once')
    wait_until(lambda:s.task()['state']=='completed','denial completes fixture turn')
    time.sleep(1)
    assert count(s)==initial+1
    s.checks.append('questions-and-real-ACP-approvals-block-without-auto-answer-or-retry')
    save(s,task,'fixture-goal-hold');initial=count(s)
    s.click_control('goal-resume',enabled=True);click(s,'composer-submit')
    wait_until(lambda:s.task()['state']=='running','goal held in real ACP turn')
    click(s,'composer-submit')
    wait_until(lambda:s.task()['state']!='running','Stop cancels current turn')
    wait_until(lambda:preference(s,key)['status']=='blocked','Stop disables continuation')
    assert count(s)==initial+1
    save(s,task,'fixture-goal-review')
    s.click_control('goal-resume',enabled=True);click(s,'composer-submit')
    wait_until(lambda:preference(s,key)['status']=='review','agent claim requires review')
    assert preference(s,key)['achievements']==[]
    fill(s,'goal-evidence','I independently reran the checks')
    click(s,'goal-achieve')
    wait_until(lambda:preference(s,key)['status']=='achieved','explicit human achievement')
    saved=preference(s,key)
    assert len(saved['achievements'])==1
    s.desktop.screenshot('goal-achievement-review',window_only=True)
    close(s);s.launch(preserve_selection=True)
    assert preference(s,key)==saved
    click(s,'goal-open')
    save(s,task,'fixture-goal-budget')
    initial=count(s)
    s.click_control('goal-resume',enabled=True)
    click(s,'new-thread')
    wait_until(lambda:'Navigation paused' in preference(s,key)['note'],'navigation disarms the goal')
    assert count(s)==initial and selection(s)==task
    s.checks.append('Send-preserves-reviewed-lease-but-navigation-disarms-without-launch')
    click(s,'goal-clear')
    wait_until(lambda:not preference(s,key)['objective'],'explicit clear')
    assert preference(s,key)['achievements']==[]
    click(s,'goal-close');click(s,'new-thread')
    wait_until(lambda:selection(s)!=task,'independent task')
    assert preference(s,'task-goal:'+selection(s)) is None
    s.checks.append('Stop-review-human-achievement-clear-restart-and-task-isolation')
    close(s)

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--binary',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    s=Scenario(p.parse_args());result={'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb'}
    try:run(s);result['status']='passed'
    except BaseException as e:
        result['error']=str(e)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:
        s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
