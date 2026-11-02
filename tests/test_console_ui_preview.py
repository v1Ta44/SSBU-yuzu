"""Render the real Console frontend with an isolated mock backend in headless Edge.
No game, server credentials, user profile or installed Console is used.
"""
from pathlib import Path
import functools
import http.server
import json
import subprocess
import tempfile
import threading
import urllib.parse

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'build' / 'network-ui-20261024'
EDGE = Path(r'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe')
BASE_MOCK = Path(r'D:\pcsmash\build\console-preview\mock.js')
AUGMENT = r"""
const originalInvoke=window.__TAURI__.core.invoke;
const scenario=new URLSearchParams(location.search).get('case') || 'yuzu';
let settings={mode:scenario==='pending'?'lobby':scenario==='upstream'?'lobby':scenario,server:'opensmash-lobby.duckdns.org:39500',join:'',rollback:true};
const actual=scenario==='pending'?'yuzu':scenario;
let game={state:'running',mode:'play',connection:actual,bridge:true,pid:123,session:'preview',uptime_s:12};
const errors=[];window.addEventListener('error',e=>errors.push(e.message));window.addEventListener('unhandledrejection',e=>errors.push(String(e.reason)));
window.__TAURI__.core.invoke=async(cmd,args)=>{
 if(cmd==='connection_get')return {...settings};
 if(cmd==='connection_set'){if(args.values.online_mode)settings.mode=args.values.online_mode;return {...settings};}
 if(cmd==='launcher_get')return {game_name:'测试玩家',romfs:'',userdata:'',romfs_default:'romfs',env:[],net:{host:'127.0.0.1',port:24872,nickname:'Player'}};
 if(cmd==='game_status')return game;
 if(cmd==='game_log')return {seq:0,lines:[]};
 if(cmd==='overview')return {found:true,package:true,root:'isolated-preview',exe:'host.exe',build_id:'preview',preflight:{fatal:[],lines:[]},sessions:[],game};
 if(cmd==='net_poll')return {connection:actual,started:true,state:0,error:0,connection_state:actual==='off'?0:2,players:actual==='off'?0:2,ping_ms:10,room:{name:'Yuzu 房间',slots:8},members:[]};
 if(cmd==='update_status')return {phase:'idle',message:'预览模式：不检查远端更新。',supported:false,busy:false,current:{version:'开发预览',build_id:'local'},config:{startup_check:false,manifest_urls:[]},check:{},latest:null,ready:null,rollback:null};
 return originalInvoke(cmd,args);
};
addEventListener('load',()=>setTimeout(async()=>{
 try {
  document.querySelector('.tile[data-page="online"]').click();
  await new Promise(r=>setTimeout(r,250));
  const current=ConnectionUI.view(game,st.net,settings);
  if(current.current!==actual)errors.push('actual connection mismatch');
  if(document.querySelector('#netForm').hidden!==(actual!=='yuzu'))errors.push('Yuzu form visibility');
  if(document.querySelector('#upstreamRollback').disabled!==(settings.mode==='yuzu'||settings.mode==='off'))errors.push('rollback availability');
  if(document.querySelector('#connectionMode').value!==settings.mode)errors.push('saved selection mismatch');
  const select=document.querySelector('#connectionMode');select.value='off';select.dispatchEvent(new Event('change'));
  await connectionQueue;
  if(settings.mode!=='off'||!document.querySelector('#upstreamRollback').disabled)errors.push('shared settings change failed');
  if(ConnectionUI.view(game,st.net,settings).current!==actual)errors.push('saved change altered current connection');
  select.value=scenario==='pending'?'lobby':scenario==='upstream'?'lobby':scenario;select.dispatchEvent(new Event('change'));await connectionQueue;
 }catch(e){errors.push(String(e.stack||e));}
 if(new URLSearchParams(location.search).get('page')==='start')document.querySelector('.tile[data-page="start"]').click();
 const style=document.createElement('style');style.textContent='*,*::before,*::after{animation:none!important;transition:none!important}';document.head.append(style);
 const tag=document.createElement('pre');tag.id='ui-check-result';tag.textContent=JSON.stringify({scenario,errors,mode:settings.mode,current:actual});tag.hidden=true;document.body.append(tag);
},1200));
"""


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    assert EDGE.is_file() and BASE_MOCK.is_file()
    class Handler(http.server.SimpleHTTPRequestHandler):
        def log_message(self,*args): pass
        def do_GET(self):
            path=urllib.parse.urlsplit(self.path).path
            if path in ('/', '/index.html'):
                body=(ROOT/'tools/console/ui/index.html').read_text(encoding='utf-8').replace('<script src="connection.js">','<script src="/__mock.js"></script><script src="connection.js">').encode()
            elif path=='/__mock.js':
                body=(BASE_MOCK.read_text(encoding='utf-8')+'\n'+AUGMENT).encode()
            else:return super().do_GET()
            self.send_response(200);self.send_header('Content-Type','text/javascript; charset=utf-8' if path.endswith('.js') else 'text/html; charset=utf-8');self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
    server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(Handler,directory=str(ROOT/'tools/console/ui')))
    threading.Thread(target=server.serve_forever,daemon=True).start()
    results=[]
    try:
        for scenario,page in [('yuzu','online'),('upstream','online'),('off','online'),('pending','online'),('yuzu','start'),('pending','start')]:
            name=scenario+'-'+page
            profile=Path(tempfile.mkdtemp(prefix='headless-',dir=OUT))
            command=[str(EDGE),'--headless','--disable-gpu','--no-first-run','--no-default-browser-check','--disable-extensions','--disable-background-networking',f'--user-data-dir={profile}', '--window-size=1440,1000','--virtual-time-budget=3500',f'--screenshot={OUT/(name+".png")}', '--dump-dom',f'http://127.0.0.1:{server.server_port}/?case={scenario}&page={page}']
            proc=subprocess.run(command,capture_output=True,timeout=35,creationflags=subprocess.CREATE_NO_WINDOW)
            (OUT/(name+'.html')).write_bytes(proc.stdout)
            (OUT/(name+'-renderer.log')).write_bytes(proc.stderr)
            import re,html
            match=re.search(r'<pre id="ui-check-result"[^>]*>(.*?)</pre>',proc.stdout.decode('utf-8','replace'),re.S)
            assert match,(scenario,'no frontend result',proc.stderr[-1000:])
            result=json.loads(html.unescape(match.group(1)));results.append(result)
            assert result['errors']==[],result
            print('PASS rendered frontend',scenario)
    finally:server.shutdown();server.server_close()
    (OUT/'ui-preview-results.json').write_text(json.dumps(results,ensure_ascii=False,indent=2),encoding='utf-8')

if __name__=='__main__':main()
