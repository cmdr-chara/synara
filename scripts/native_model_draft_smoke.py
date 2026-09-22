#!/usr/bin/env python3
"""Provider tabs, explicit connect, favorites and durable drafts on private Xvfb."""
import argparse, json, sqlite3, time
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, key_edge, task_count, event_cursor, prompt_finished
from native_controls_smoke import option, config_count
from native_studio_settings_smoke import mode
def preference(s, key):
    path=s.data/'native-workspace.sqlite3'
    with sqlite3.connect(path.as_uri()+'?mode=ro',uri=True) as db:
        row=db.execute('SELECT data FROM preferences WHERE key=?',(key,)).fetchone()
        return json.loads(row[0]) if row else None
def search(s,text):
    s.click_control('choice-search'); s.desktop.key('a',('Control_L',)); s.desktop.text(text)
def close(s):
    s.desktop.request_close(); wait_until(lambda:s.process.poll() is not None,'draft-safe close')
    assert s.process.returncode==0
    s.log.close(); s.log=None
def run(s):
    s.launch(); ui=s.desktop; original=selection(s); events=s.events()
    s.click_control('agent-picker'); s.click_control('model-source',slot=2)
    search(s,'beta'); key_edge(ui,'Return',True)
    assert s.task()['agent_id']=='alpha'
    key_edge(ui,'Return',False)
    wait_until(lambda:s.task()['agent_id']=='beta','explicit beta selection')
    assert s.events()==events
    s.checks.append('provider-tabs-do-not-start-agents-and-selection-is-release-only')
    s.click_control('agent-picker'); search(s,'Connect'); ui.key('Return')
    wait_until(lambda:option(s,'model')=='beta','explicit connection loads advertised models')
    assert not any(e['type']=='prompt_started' for e in s.events())
    s.checks.append('connect-from-picker-loads-capabilities-without-sending')
    s.click_control('model-picker'); search(s,'alternate'); before=config_count(s)
    s.click_control('model-star',slot=0)
    wait_until(lambda:preference(s,'model-favorites') and len(preference(s,'model-favorites')['entries'])==1,'favorite persisted')
    assert config_count(s)==before
    s.click_control('model-source',slot=0); ui.screenshot('starred-models')
    key_edge(ui,'Return',True); assert config_count(s)==before
    key_edge(ui,'Return',False)
    wait_until(lambda:option(s,'model')=='alternate','starred model acknowledged')
    assert config_count(s)==before+1
    s.checks.append('stars-persist-without-changing-model-and-starred-selection-applies-once')
    s.click_control('composer-input'); ui.text('draft first line'); ui.key('Return',('Shift_L',)); ui.text('second line')
    draft='draft first line\nsecond line'
    wait_until(lambda:(preference(s,'task-draft:'+original) or {}).get('text')==draft,'debounced multiline draft')
    before=s.events(); close(s); s.launch(preserve_selection=True); ui=s.desktop
    s.click_control('composer-input'); assert ui.copy_input()==draft
    assert s.events()==before and len(preference(s,'model-favorites')['entries'])==1
    s.checks.append('draft-and-favorites-survive-restart-without-agent-autostart')
    ui.focus(); ui.key('End'); ui.text(' final edit')
    # Close without waiting for the debounce. The window must drain the write.
    close(s); s.launch(preserve_selection=True); ui=s.desktop
    s.click_control('composer-input'); assert ui.copy_input()==draft+' final edit'
    s.checks.append('close-flushes-pending-draft-before-exit')
    s.click_control('new-thread'); wait_until(lambda:task_count(s)==2,'independent chat')
    chat=selection(s); s.click_control('composer-input'); ui.text('chat draft')
    mode(s,True)
    # Hubs are optional now. Opening their home must not create/select work.
    assert selection(s)==chat and task_count(s)==2
    s.click_control('hub-create'); ui.text('Draft isolation Hub')
    s.click_control('hub-save'); wait_until(lambda:selection(s)!=chat,'explicit Hub creation')
    s.click_control('hub-home-thread',slot=0)
    studio=selection(s); s.click_control('composer-input'); ui.text('studio draft')
    mode(s,False); wait_until(lambda:selection(s)==chat,'return to chat')
    s.click_control('composer-input'); observed=ui.copy_input(); assert observed=='chat draft', (observed, selection(s), chat, preference(s,'task-draft:'+chat))
    wait_until(lambda:(preference(s,'task-draft:'+studio) or {}).get('text')=='studio draft','Studio draft isolated')
    close(s); s.launch(preserve_selection=True); ui=s.desktop
    s.click_control('composer-input'); observed=ui.copy_input(); assert observed=='chat draft', (observed, selection(s), chat, preference(s,'task-draft:'+chat))
    s.checks.append('optional-Hub-and-standalone-drafts-stay-separate-across-switching-and-restart')
    before=event_cursor(s,chat); ui.focus(); ui.key('a',('Control_L',)); s.prompt('hello'); wait_until(lambda:prompt_finished(s,chat,before),'selected chat completion')
    wait_until(lambda:(preference(s,'task-draft:'+chat) or {}).get('text')=='','accepted-prompt-clears-only-sent-draft')
    assert (preference(s,'task-draft:'+studio) or {}).get('text')=='studio draft'
    ui.screenshot('persisted-chat-transcript')
    s.checks.append('accepted-user-echo-clears-sent-draft-without-erasing-other-chats')
def main():
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    s=Scenario(p.parse_args());result={'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb'}
    try: run(s);result['status']='passed'
    except BaseException as e:
        result['error']=str(e)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:
        s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
