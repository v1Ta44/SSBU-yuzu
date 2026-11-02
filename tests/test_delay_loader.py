"""Run the actual optional delay loader from a Unicode installation directory."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
PREAMBLE = r"""
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <wchar.h>
typedef int HookApi;
static const HookApi g_hook_api=123;
static HMODULE g_delay_mod;
"""
PLUGIN = r"""
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
__declspec(dllexport) void mod_init(const int *api) {
    const wchar_t *marker=_wgetenv(L"DELAY_TEST_MARKER");
    FILE *f=marker?_wfopen(marker,L"w, ccs=UTF-8"):NULL;
    if(!f)return;
    const wchar_t *config=_wgetenv(L"SSBU_NET_DELAY_CONFIG");
    fwprintf(f,L"api=%d\nconfig=%ls\n",*api,config?config:L"missing");fclose(f);
}
"""

class DelayLoaderTests(unittest.TestCase):
    def test_unicode_install_and_disabled_build(self):
        compiler=os.environ.get("RECOMP_CLANG") or shutil.which("clang")
        if not compiler:self.skipTest("clang unavailable")
        source=(ROOT/"fighter_sdk/hook_host.c").read_text(encoding="utf8")
        a=source.index("static void hook_load_delay(void)")
        b=source.index("static void hook_load_mods(void)",a)
        with tempfile.TemporaryDirectory(prefix="delay-loader-") as temp:
            root=Path(temp)/"中文安装目录";work=root/"game";plugins=work/"plugins";plugins.mkdir(parents=True)
            c=Path(temp)/"loader.c";c.write_text(PREAMBLE+source[a:b]+"\nint main(void){hook_load_delay();return 0;}\n",encoding="utf8")
            dll=Path(temp)/"plugin.c";dll.write_text(PLUGIN,encoding="utf8")
            def build(args):
                r=subprocess.run([compiler,*args],capture_output=True,text=True)
                self.assertEqual(r.returncode,0,r.stdout+r.stderr)
            build(["-shared",str(dll),"-o",str(plugins/"latency_slider.dll")])
            env={k:v for k,v in os.environ.items() if k.upper() not in ("SSBU_NET_DELAY_CONFIG","DELAY_TEST_MARKER")}
            marker=root/"加载结果.txt";env["DELAY_TEST_MARKER"]=str(marker)
            for enabled in (1,0):
                exe=work/("enabled.exe" if enabled else "disabled.exe")
                build(["-O1","-DSSBU_ENABLE_NET_DELAY="+str(enabled),str(c),"-o",str(exe)])
                if marker.exists():marker.unlink()
                result=subprocess.run([str(exe)],env=env,capture_output=True,timeout=10)
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertEqual(marker.exists(),bool(enabled))
                if enabled:
                    text=marker.read_text(encoding="utf-8-sig");self.assertIn("api=123",text)
                    self.assertIn("config="+str(work)+r"\..\config\online-delay.ini",text)
                    self.assertTrue((root/"config").is_dir())
            explicit=root/"自选配置.ini";env["SSBU_NET_DELAY_CONFIG"]=str(explicit)
            subprocess.run([str(work/"enabled.exe")],env=env,check=True,timeout=10)
            self.assertIn("config="+str(explicit),marker.read_text(encoding="utf-8-sig"))
            marker.unlink();(plugins/"latency_slider.dll").rename(plugins/"disabled.dll")
            r=subprocess.run([str(work/"enabled.exe")],env=env,capture_output=True,timeout=10)
            self.assertEqual(r.returncode,0);self.assertIn(b"enabled extension missing",r.stdout);self.assertFalse(marker.exists())
            print("PASS real loader: Unicode DLL/config paths, explicit config, disabled build, missing extension")

if __name__=="__main__":unittest.main()
