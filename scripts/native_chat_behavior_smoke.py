#!/usr/bin/env python3
"""Persisted chat key behavior and timestamp display with real native input."""
import argparse, json, re
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_model_draft_smoke import preference, close
def settings(s): return preference(s,'settings') or {}
def log(s): return re.sub(r'\x1b\[[0-9;]*[A-Za-z]','',Path(s.log.name).read_text())
def run(s):
    s.launch(); ui=s.desktop
    before=s.prompt('hello'); s.finished(before)
    assert s.control_bounds('message-timestamp')
    ui.key('6',('Control_L',)); s.click_control('behavior')
    s.click_control('chat-send-enter'); wait_until(lambda:settings(s).get('chat',{}).get('send_on_enter') is False,'saved Enter preference')
    s.click_control('chat-timestamps'); wait_until(lambda:settings(s).get('chat',{}).get('show_timestamps') is False,'saved timestamp preference')
    ui.screenshot('chat-behavior-settings')
    start=len(log(s)); ui.key('1',('Control_L',)); s.click_control('composer-input')
    assert 'control="message-timestamp"' not in log(s)[start:]
    before=s.events(); ui.text('first'); ui.key('Return'); ui.text('second')
    assert ui.copy_input()=='first\nsecond'
    assert s.events()==before
    s.checks.append('disabled-enter-inserts-newline-without-starting-a-turn')
    ui.focus(); ui.key('a',('Control_L',)); ui.text('hello'); before=len(s.events()); ui.key('Return',('Control_L',)); s.finished(before)
    s.checks.append('control-enter-sends-through-the-existing-controller')
    s.checks.append('timestamp-toggle-changes-native-rendering-not-transcript-events')
    before=s.events(); close(s); s.launch(preserve_selection=True); ui=s.desktop
    assert s.events()==before
    assert settings(s)['chat']=={'send_on_enter':False,'show_timestamps':False}
    s.click_control('composer-input'); ui.text('kept'); ui.key('Return'); ui.text('draft')
    assert ui.copy_input()=='kept\ndraft' and s.events()==before
    s.checks.append('chat-preferences-and-key-policy-survive-restart-without-autostart')
    ui.key('6',('Control_L',)); s.click_control('behavior'); s.click_control('settings-restore')
    wait_until(lambda:settings(s).get('chat')=={'send_on_enter':True,'show_timestamps':True},'chat-only defaults')
    ui.key('1',('Control_L',)); s.click_control('composer-input'); ui.key('a',('Control_L',)); ui.text('hello')
    before=len(s.events()); ui.key('Return'); s.finished(before)
    s.checks.append('restore-chat-defaults-reactivates-enter-send-and-timestamps')
def main():
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    s=Scenario(p.parse_args()); result={'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb'}
    try:run(s);result['status']='passed'
    except BaseException as e:
        result['error']=str(e)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
