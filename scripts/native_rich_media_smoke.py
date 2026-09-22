#!/usr/bin/env python3
"""Actual clipboard, durable ACP images and GPUI rendering on private Xvfb."""
import argparse
import base64
import json
import os
import subprocess
from pathlib import Path
from PIL import Image
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import close, preference
from native_integrations_smoke import click, fresh_probe


def run(s):
    image_path=s.output/'owned-image.png'
    image=Image.new('RGB',(96,64))
    for x in range(96):
        for y in range(64): image.putpixel((x,y),(x*2,y*3,180))
    image.save(image_path)
    original=image_path.read_bytes()
    s.launch();task=selection(s)
    s.click_control('composer-input')
    clip=subprocess.Popen(['xclip','-selection','clipboard','-t','image/png','-i',str(image_path),'-loops','0','-quiet'],
        env={**os.environ,'DISPLAY':s.desktop.name},stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    try:
        s.desktop.focus();s.desktop.key('v',('Control_L',))
        wait_until(lambda:(preference(s,'task-attachments:'+task) or {}).get('pending'),'native image clipboard import')
    finally:
        clip.terminate();clip.wait(timeout=5)
    assert not any(e['type']=='prompt_started' for e in s.events())
    pending=preference(s,'task-attachments:'+task)['pending']
    assert pending[0]['info']['source']=='uploaded'
    s.checks.append('native-clipboard-creates-durable-upload-without-sending')
    before=s.prompt('transcript-media');s.finished(before)
    images=[e for e in s.events() if e['type']=='image_message']
    assert len(images)==2 and [e['image']['source'] for e in images]==['uploaded','agent_returned']
    assert all(base64.b64decode(e['image']['base64'],validate=True)==original for e in images)
    wait_until(lambda:s.control_bounds('transcript-image-ready',1),'agent image inline rendering')
    s.desktop.screenshot('inline-transcript-images',window_only=True)
    fresh_probe(s,'transcript-image-expanded',lambda:click(s,'transcript-image-expand',slot=1))
    s.desktop.screenshot('expanded-transcript-image',window_only=True)
    s.checks.append('exact-upload-and-agent-returned-bytes-persist-with-distinct-provenance-and-expand')
    # Recent is deliberately evictable and must not own sent-image persistence.
    click(s,'recent-attachments');click(s,'forget-recent-attachments')
    wait_until(lambda:not (preference(s,'task-attachments:'+task) or {}).get('recent'),'recent snapshots cleared')
    events=s.events();close(s);s.launch(preserve_selection=True)
    wait_until(lambda:s.control_bounds('transcript-image-ready',1),'restart reconstructs image preview')
    assert s.events()==events
    s.desktop.screenshot('restored-media-without-recent',window_only=True)
    s.checks.append('restart-restores-originals-after-recent-clearing-with-no-agent-autostart')
    before=s.prompt('corrupt-media');s.finished(before)
    assert any(e['type']=='notice' and 'Image unavailable' in e['message'] for e in s.events()[before:])
    assert s.has_text('A corrupt image did not stop',before)
    assert len([e for e in s.events() if e['type']=='image_message'])==2
    s.checks.append('malformed-provider-image-fails-softly-without-storing-corruption')
    events=s.events();click(s,'new-thread')
    wait_until(lambda:task_count(s)==2 and selection(s)!=task,'second task')
    # A newly loaded empty task must not reuse cached images from its predecessor.
    assert s.events()==events
    s.desktop.screenshot('separate-task-no-media',window_only=True)
    s.checks.append('task-switch-preserves-history-and-does-not-send-or-inherit-media')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--fixture',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    s=Scenario(parser.parse_args());result={'status':'failed','checks':s.checks,'platform':'Linux/X11/private Xvfb / owned clipboard and ACP fixture'}
    try:run(s);result['status']='passed'
    except BaseException as error:
        result['error']=str(error)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:
        s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
