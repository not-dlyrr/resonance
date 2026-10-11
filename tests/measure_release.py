import sys, pathlib, json, time, urllib.request, subprocess, os, statistics
sys.path.insert(0,str(pathlib.Path(__file__).parent/'pythonlibs'))
import psutil, websocket
def targets(port):
    with urllib.request.urlopen(f'http://127.0.0.1:{port}/json',timeout=2) as r:return json.load(r)
def evaluate(port,expression):
    pages=[p for p in targets(port) if p.get('type')=='page' and 'devtools' not in p.get('url','')]
    p=next((p for p in pages if 'xpui.app.spotify.com' in p.get('url','')),pages[0])
    w=websocket.create_connection(p['webSocketDebuggerUrl'],timeout=8,suppress_origin=True)
    w.send(json.dumps({'id':1,'method':'Runtime.evaluate','params':{'expression':expression,'returnByValue':True,'awaitPromise':True}}))
    while True:
        r=json.loads(w.recv())
        if r.get('id')==1:break
    w.close()
    if 'exceptionDetails' in r.get('result',{}):raise RuntimeError(r['result']['exceptionDetails'])
    return r.get('result',{}).get('result',{}).get('value')

def screenshot(port,path,width=None,height=None):
    import base64
    p=next(p for p in targets(port) if p.get('type')=='page' and 'devtools' not in p.get('url',''))
    w=websocket.create_connection(p['webSocketDebuggerUrl'],timeout=8,suppress_origin=True)
    if width:
        w.send(json.dumps({'id':2,'method':'Emulation.setDeviceMetricsOverride','params':{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False}}))
        while True:
            m=json.loads(w.recv())
            if m.get('id')==2:break
        time.sleep(.4)
    w.send(json.dumps({'id':1,'method':'Page.captureScreenshot','params':{'format':'png'}}))
    while True:
        r=json.loads(w.recv())
        if r.get('id')==1:break
    w.close();pathlib.Path(path).write_bytes(base64.b64decode(r['result']['data']))

def metrics(port,width,height):
    p=next(p for p in targets(port) if p.get('type')=='page' and 'devtools' not in p.get('url',''))
    w=websocket.create_connection(p['webSocketDebuggerUrl'],timeout=8,suppress_origin=True)
    method='Emulation.setDeviceMetricsOverride' if width else 'Emulation.clearDeviceMetricsOverride'
    params={'width':width,'height':height,'deviceScaleFactor':1,'mobile':False} if width else {}
    w.send(json.dumps({'id':1,'method':method,'params':params}))
    while True:
        r=json.loads(w.recv())
        if r.get('id')==1:break
    w.close()
    if 'error' in r:raise RuntimeError(r['error'])
def processes(exe):
    exe=os.path.normcase(os.path.abspath(exe));roots=[]
    for p in psutil.process_iter(['exe']):
        try:
            if p.info['exe'] and os.path.normcase(p.info['exe'])==exe:roots.append(p)
        except psutil.Error:pass
    out={}
    for p in roots:
        try:
            for c in [p]+p.children(recursive=True):out[c.pid]=c
        except psutil.Error:pass
    return list(out.values())
def stop(exe):
    for p in processes(exe):
        try:p.terminate()
        except psutil.Error:pass
    psutil.wait_procs(processes(exe),timeout=5)
def measure(exe,seconds=8):
    p=processes(exe);before={}
    if not p:raise RuntimeError('No matching process tree; use the actual running executable path')
    for x in p:
        try:before[x.pid]=sum(x.cpu_times()[:2])
        except psutil.Error:pass
    start=time.perf_counter();time.sleep(seconds);after=processes(exe);cpu=0;rss=0
    for x in after:
        try:cpu+=max(0,sum(x.cpu_times()[:2])-before.get(x.pid,sum(x.cpu_times()[:2])));rss+=x.memory_info().rss
        except psutil.Error:pass
    return {'idle_cpu_percent_total_machine':cpu/(time.perf_counter()-start)/psutil.cpu_count()*100,'combined_working_set_mb':rss/1024/1024,'processes':len(after),'window_seconds':seconds}
def bench(label,exe,port,manager=False):
    results=[]
    for i in range(3):
        stop(exe);time.sleep(.3);start=time.perf_counter();env=os.environ.copy()
        if manager:env['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS']=f'--remote-debugging-port={port}'
        args=[exe] if manager else [exe,f'--remote-debugging-port={port}']
        subprocess.Popen(args,env=env)
        ready=None
        for _ in range(160):
            try:
                condition="document.querySelectorAll('.theme-card').length>=3 && document.querySelector('.engine-status')?.textContent.includes('1.3.4')" if manager else "!!document.querySelector('[data-testid=control-button-playpause]')"
                if evaluate(port,condition):ready=time.perf_counter()-start;break
            except Exception:pass
            time.sleep(.1)
        if ready is None:raise RuntimeError(f'{label}: usable UI not detected within timeout')
        if not manager:
            # Comparable paused playback on Home; no user music titles are recorded.
            evaluate(port,"(()=>{const b=document.querySelector('[data-testid=control-button-playpause]');if(b?.getAttribute('aria-label')?.toLowerCase().includes('pause'))b.click();const h=document.querySelector('[aria-label=Home]');h?.click();return true})()")
        time.sleep(4)
        if not manager:
            playing=evaluate(port,"document.querySelector('[data-testid=control-button-playpause]')?.getAttribute('aria-label')?.toLowerCase().includes('pause')")
            if playing: raise RuntimeError('Playback changed during idle preparation; benchmark aborted')
        row={'run':i+1,'usable_startup_seconds':ready,**measure(exe)};results.append(row);print(json.dumps(row),flush=True)
    output={'label':label,'runs':results,'method':'Warm filesystem cache; 3 process restarts; usable DOM controls; 4s settle; 8s idle measurement; summed process-tree working sets (shared pages may be counted multiple times); CPU normalized to logical processor count. Spotify Home paused. CDP instrumentation enabled.'}
    path=pathlib.Path('docs/measurements');path.mkdir(parents=True,exist_ok=True);(path/(label+'.json')).write_text(json.dumps(output,indent=2));return output
if __name__=='__main__':
    cmd=sys.argv[1]
    if cmd=='eval':print(json.dumps(evaluate(int(sys.argv[2]),sys.argv[3])))
    elif cmd=='screenshot':screenshot(int(sys.argv[2]),sys.argv[3])
    elif cmd=='viewport-shot':screenshot(int(sys.argv[2]),sys.argv[3],int(sys.argv[4]),int(sys.argv[5]))
    elif cmd=='metrics':metrics(int(sys.argv[2]),int(sys.argv[3]),int(sys.argv[4]))
    elif cmd=='stop':stop(sys.argv[2])
    elif cmd=='measure':print(json.dumps(measure(sys.argv[2],float(sys.argv[3]) if len(sys.argv)>3 else 8)))
    elif cmd=='bench':bench(sys.argv[2],sys.argv[3],int(sys.argv[4]),len(sys.argv)>5 and sys.argv[5]=='manager')
