#!/usr/bin/env python3
"""Real one-window X11 capture and durable native composer intake on private Xvfb."""
import argparse
import ctypes as C
import io
import json
import os
import time
from pathlib import Path
from PIL import Image
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import close, preference
from native_integrations_smoke import click, fresh_probe


class ClassHint(C.Structure):
    _fields_=[('res_name',C.c_char_p),('res_class',C.c_char_p)]


class OwnedWindow:
    """Only this fixture's private display and newly created window are mutated."""
    def __init__(self,desktop):
        self.ui=desktop;x=desktop.x;ptr=C.c_void_p;u=C.c_ulong;i=C.c_int
        bindings={
            'XCreateSimpleWindow':([ptr,u,i,i,C.c_uint,C.c_uint,C.c_uint,u,u],u),
            'XStoreName':([ptr,u,C.c_char_p],i),'XMapWindow':([ptr,u],i),'XDestroyWindow':([ptr,u],i),
            'XChangeProperty':([ptr,u,u,u,i,i,C.POINTER(C.c_ubyte),i],i),
            'XSetClassHint':([ptr,u,C.POINTER(ClassHint)],i),'XSync':([ptr,i],i)}
        for name,(args,result) in bindings.items():getattr(x,name).argtypes=args;getattr(x,name).restype=result
        self.window=x.XCreateSimpleWindow(desktop.display,desktop.root,10,10,80,60,0,0,0x2871B8)
        assert self.window
        hint=ClassHint(b'appsnap-fixture',b'AppSnapFixture');x.XSetClassHint(desktop.display,self.window,C.byref(hint))
        pid=C.c_ulong(os.getpid());self.property('_NET_WM_PID','CARDINAL',32,C.cast(C.byref(pid),C.POINTER(C.c_ubyte)),1)
        self.rename('!AppSnap owned fixture')
        x.XMapWindow(desktop.display,self.window);x.XSync(desktop.display,0)
    def property(self,name,kind,bits,data,size):
        x=self.ui.x;d=self.ui.display
        x.XChangeProperty(d,self.window,x.XInternAtom(d,name.encode(),0),x.XInternAtom(d,kind.encode(),0),bits,0,data,size)
    def rename(self,name):
        raw=name.encode();buf=(C.c_ubyte*len(raw)).from_buffer_copy(raw)
        self.property('_NET_WM_NAME','UTF8_STRING',8,buf,len(raw))
        self.ui.x.XStoreName(self.ui.display,self.window,raw);self.ui.x.XSync(self.ui.display,0)
    def close(self):
        if self.window:
            self.ui.x.XDestroyWindow(self.ui.display,self.window);self.ui.x.XSync(self.ui.display,0);self.window=None


def pending(s,task):return (preference(s,'task-attachments:'+task) or {}).get('pending',[])

def run(s):
    s.launch();task=selection(s);events=s.events();target=OwnedWindow(s.desktop)
    try:
        click(s,'appsnap-toggle');wait_until(lambda:s.control_bounds('appsnap-setup'),'explicit setup')
        assert not s.control_bounds('appsnap-window') and not pending(s,task) and s.events()==events
        s.checks.append('open-does-not-discover-capture-or-assume-os-permission')
        click(s,'appsnap-setup');wait_until(lambda:s.control_bounds('appsnap-window',0),'real X11 window discovery',25)
        fresh_probe(s,'appsnap-reviewed',lambda:click(s,'appsnap-window',slot=0))
        assert not pending(s,task) and s.events()==events
        s.desktop.screenshot('reviewed-window-without-capture',window_only=True)
        target.rename('!AppSnap changed after review')
        fresh_probe(s,'appsnap-error',lambda:click(s,'appsnap-capture'))
        assert not pending(s,task) and s.events()==events
        s.checks.append('stale-reviewed-title-rejects-capture-before-any-attachment-or-send')
        click(s,'appsnap-setup');time.sleep(0.7)
        click(s,'appsnap-window',slot=0);click(s,'appsnap-capture')
        saved=wait_until(lambda:pending(s,task),'real bounded window capture attached',25)
        assert len(saved)==1 and saved[0]['info']['source']=='app_snap'
        original=bytes.fromhex(saved[0]['hex']);image=Image.open(io.BytesIO(original)).convert('RGB')
        assert image.size==(80,60) and image.getpixel((40,30))==(40,113,184),(image.size,image.getpixel((40,30)))
        assert s.events()==events
        s.desktop.screenshot('captured-window-pending-not-sent',window_only=True)
        s.checks.append('exact-selected-window-pixels-not-desktop-enter-existing-pending-owner-without-send')
        close(s);s.launch(preserve_selection=True)
        assert pending(s,task)==saved and s.events()==events
        assert not s.control_bounds('appsnap-panel')
        click(s,'appsnap-toggle');assert not s.control_bounds('appsnap-window') and not s.control_bounds('appsnap-capture')
        s.checks.append('restart-preserves-pending-original-but-not-window-selection-or-capture-consent')
        click(s,'appsnap-setup');time.sleep(0.7);click(s,'appsnap-window',slot=0)
        click(s,'new-thread');wait_until(lambda:task_count(s)==2 and selection(s)!=task,'independent second task')
        assert not pending(s,selection(s)) and pending(s,task)==saved and s.events()==events
        # On re-opening in the replacement task, setup is required and no reviewed
        # window or bytes from the previous task can become its attachment.
        fresh_probe(s,'appsnap-panel',lambda:click(s,'appsnap-toggle'))
        assert not pending(s,selection(s));s.desktop.screenshot('replacement-task-without-capture-consent',window_only=True)
        s.checks.append('task-replacement-revokes-window-review-and-does-not-cross-attach-or-send')
    finally:target.close()


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--fixture',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    s=Scenario(parser.parse_args());result={'status':'failed','checks':s.checks,'platform':'Linux/X11 / owned 80x60 window / real xwininfo, xprop and ImageMagick'}
    try:run(s);result['status']='passed'
    except BaseException as error:
        result['error']=str(error)
        if s.process and s.process.poll() is None:s.desktop.screenshot('failure')
        raise
    finally:
        s.close();(s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
