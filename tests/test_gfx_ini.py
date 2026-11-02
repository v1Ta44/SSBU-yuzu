"""Compile the actual F1 settings writer and re-read its files in a fresh runtime."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
HARNESS = r"""
#include <stdio.h>
#include <stdlib.h>
#include "gfx_options.h"
int main(int argc,char **argv) {
    if(argc<3)return 2;
    _putenv_s("SSBU_GRAPHICS_CONFIG",argv[1]);
    if(argc==5)return gfx_ini_set(argv[3],argv[4])?0:1;
    const GfxOptions *o=gfx_options();
    printf("READ present=%d latency=%d\n",o->present,gfx_latency());
    return 0;
}
"""

class GfxIniTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        compiler=os.environ.get("RECOMP_CLANG") or shutil.which("clang")
        if not compiler:raise unittest.SkipTest("clang unavailable")
        cls.temp=tempfile.TemporaryDirectory(prefix="gfx-ini-")
        cls.root=Path(cls.temp.name)
        source=cls.root/"harness.c";source.write_text(HARNESS,encoding="utf8")
        cls.exe=cls.root/"gfx_ini.exe"
        result=subprocess.run([compiler,"-O1","-I",str(ROOT/"runtime"),str(source),str(ROOT/"runtime/gfx_options.c"),"-o",str(cls.exe)],capture_output=True,text=True)
        if result.returncode:
            cls.temp.cleanup()
            raise AssertionError(result.stdout+result.stderr)
        cls.env={k:v for k,v in os.environ.items() if not k.upper().startswith(("SSBU_","RECOMP_"))}

    @classmethod
    def tearDownClass(cls):cls.temp.cleanup()

    def setUp(self):
        self.config=self.root/(self._testMethodName+".ini")

    def change(self,before,key,value):
        self.config.write_bytes(before)
        result=subprocess.run([str(self.exe),str(self.config),"set",key,value],env=self.env,capture_output=True,timeout=10)
        self.assertEqual(result.returncode,0,(result.stdout+result.stderr).decode("utf8","replace"))
        self.assertFalse(self.config.with_suffix(".ini.tmp").exists())
        return self.config.read_bytes()

    def read(self):
        result=subprocess.run([str(self.exe),str(self.config),"read"],env=self.env,capture_output=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        return result.stdout.decode("utf8","replace")

    def test_duplicate_graphics_value_survives_restart(self):
        before=b"vsync = auto\n; retain this comment\ncustom = keep\nvsync = auto\n"
        expected=b"vsync = off\r\n; retain this comment\ncustom = keep\nvsync = off\r\n"
        self.assertEqual(self.change(before,"vsync","off"),expected)
        self.assertIn("READ present=2",self.read())

    def test_duplicate_latency_case_and_whitespace(self):
        before=b"  LaTeNcY\t = normal\r\n# latency=lowest\nlatency_extra=keep\n\tLATENCY=normal"
        expected=b"latency = low\r\n# latency=lowest\nlatency_extra=keep\nlatency = low\r\n"
        self.assertEqual(self.change(before,"latency","low"),expected)
        self.assertIn("latency=1",self.read())

    def test_duplicate_online_values_agree_for_first_and_last_readers(self):
        out=self.change(b"online_mode = yuzu\nrenderer=vulkan\nonline_mode = yuzu\n","online_mode","off")
        self.assertEqual(out,b"online_mode = off\r\nrenderer=vulkan\nonline_mode = off\r\n")

    def test_single_key_preserves_unrelated_bytes(self):
        before=b"; custom comment\n[future]\nuser_unknown = \xe6\xb5\x8b\xe8\xaf\x95\nvsync=auto\n# end\n"
        self.assertEqual(self.change(before,"vsync","off"),before.replace(b"vsync=auto\n",b"vsync = off\r\n"))

    def test_missing_key_appended_after_unterminated_line(self):
        self.assertEqual(self.change(b"custom=keep","vsync","off"),b"custom=keep\r\nvsync = off\r\n")

    def test_empty_file_and_short_lines(self):
        self.assertEqual(self.change(b"","online_mode","off"),b"online_mode = off\r\n")
        before=b"\n;\n#\nx\n[future]\nonline_mode_extra=keep\n"
        self.assertEqual(self.change(before,"online_mode","off"),before+b"online_mode = off\r\n")

    def test_many_duplicates_with_long_values_grow_output(self):
        value="server-"+"x"*4096
        before=(b"x=\n"*2000)+b"custom=keep\n"
        expected=(("x = "+value+"\r\n").encode()*2000)+b"custom=keep\n"
        self.assertEqual(self.change(before,"x",value),expected)

    def test_many_duplicates_can_shrink_output(self):
        before=(("x = "+"v"*500+"\n").encode()*1000)+b"z=keep"
        self.assertEqual(self.change(before,"x",""),b"x = \r\n"*1000+b"z=keep")

if __name__=="__main__":unittest.main(verbosity=2)
