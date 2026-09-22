#!/usr/bin/env python3
"""Native file selection and task-owned comments. Fixture file edits only, no SQL writes."""
import argparse
import json
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, click
from native_navigation_smoke import selection
from native_project_import_smoke import task_events
from native_model_draft_smoke import preference, close


def saved(s, task):
    return preference(s, 'task-inline-comments:' + task) or {}


def editor(s):
    s.desktop.key('2', ('Control_L',))
    wait_until(lambda: s.control_bounds('file-row', 0), 'file row')
    s.click_control('file-row', slot=0)
    wait_until(lambda: s.control_bounds('editor-input'), 'editor')


def comment(s, text):
    s.click_control('editor-input')
    s.desktop.key('Home', ('Control_L',))
    s.desktop.key('End', ('Shift_L',))
    click(s, 'inline-review-open')
    fill(s, 'inline-review-input', text)
    click(s, 'inline-review-save')


def run(s):
    s.launch(); task=selection(s)
    fill(s,'composer-input','Keep this request')
    wait_until(lambda:(preference(s,'task-draft:'+task) or {}).get('text')=='Keep this request','source draft')
    before=task_events(s,task)
    editor(s);comment(s,'Review this original line')
    wait_until(lambda:len(saved(s,task).get('items',[]))==1,'first saved comment')
    item=saved(s,task)['items'][0]
    assert item['path']=='document.txt' and item['first_line']==item['last_line']==1
    assert 'original text' in item['context'] and len(item['version'])==64
    comment(s,'Keep the second comment too')
    wait_until(lambda:len(saved(s,task).get('items',[]))==2,'second saved comment')
    s.desktop.screenshot('inline-comments-reviewed',window_only=True)
    assert task_events(s,task)==before
    s.checks.append('native-selection-retains-file-line-version-and-multiple-comments-without-send')

    close(s);s.launch(preserve_selection=True)
    assert len(saved(s,task)['items'])==2 and task_events(s,task)==before
    editor(s);click(s,'inline-review-open')
    # This opens an empty edit. Appending saved comments does not send that edit.
    s.document.write_text('changed outside the editor\n')
    click(s,'inline-review-append')
    wait_until(lambda:s.control_bounds('inline-review-error'),'changed file rejection')
    assert preference(s,'task-draft:'+task)['text']=='Keep this request'
    s.checks.append('restart-retains-comments-and-stale-file-version-blocks-draft-insertion')

    renamed=s.project/'renamed.txt';s.document.rename(renamed)
    click(s,'inline-review-append')
    wait_until(lambda:s.control_bounds('inline-review-error'),'missing or renamed rejection')
    assert preference(s,'task-draft:'+task)['text']=='Keep this request'
    renamed.rename(s.document);s.document.write_text('original text\n')
    click(s,'inline-review-append')
    wait_until(lambda:'Reviewed inline file comments' in (preference(s,'task-draft:'+task) or {}).get('text',''),'review context appended')
    draft=preference(s,'task-draft:'+task)['text']
    assert draft.startswith('Keep this request\n\n')
    assert all(text in draft for text in ['Review this original line','Keep the second comment too','document.txt','Lines: 1-1','UNTRUSTED'])
    assert task_events(s,task)==before and selection(s)==task
    s.desktop.screenshot('inline-comments-unsent',window_only=True)
    s.checks.append('deleted-renamed-files-fail-closed-and-restored-reviewed-versions-append-only-to-source-draft')

    close(s);s.launch(preserve_selection=True)
    assert preference(s,'task-draft:'+task)['text']==draft and task_events(s,task)==before
    click(s,'new-thread');wait_until(lambda:selection(s)!=task,'different task')
    assert not saved(s,selection(s)).get('items',[])
    assert len(saved(s,task)['items'])==2
    s.checks.append('combined-draft-survives-restart-and-comments-do-not-leak-to-another-task')
    close(s)


def main():
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    s=Scenario(p.parse_args());result={'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb'}
    try:run(s);result['status']='passed'
    except BaseException as error:
        result['error']=str(error)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:
        s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
