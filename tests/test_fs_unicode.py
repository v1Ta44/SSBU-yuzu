"""Run the real Windows filesystem adapters on isolated Unicode fixtures, without game/GPU."""
from pathlib import Path
import os,subprocess,tempfile,shutil
ROOT=Path(__file__).resolve().parents[1]
def main():
 out=ROOT/'build/skyline-unicode-20261025';out.mkdir(parents=True,exist_ok=True)
 clang=os.environ.get('RECOMP_CLANG') or shutil.which('clang') or str(ROOT/'deps/llvm/bin/clang.exe')
 exe=out/'fs_unicode.exe'
 r=subprocess.run([clang,'-O1','-w','-DSSBU_ENABLE_YUZU=0','-I',str(ROOT/'runtime'),str(ROOT/'tests/fs_unicode.c'),str(ROOT/'runtime/nnfs.c'),str(ROOT/'runtime/nnfsdir.c'),'-o',str(exe)],capture_output=True)
 (out/'fs-build.log').write_bytes(r.stdout+r.stderr)
 if r.returncode:raise RuntimeError((r.stdout+r.stderr).decode('utf-8','replace'))
 with tempfile.TemporaryDirectory(prefix='fixture-',dir=out)as tmp:
  root=Path(tmp)/'根目录_日本語_🎮';root.mkdir();user=Path(tmp)/'独立存档_日本語';user.mkdir()
  env={k:v for k,v in os.environ.items()if not k.upper().startswith('SSBU_')}
  env.update(SSBU_FS_TEST_ROOT=str(root),SSBU_USERDATA=str(user),SSBU_ROMFS=str(root),SSBU_NICKNAME='Validation')
  r=subprocess.run([str(exe)],env=env,capture_output=True,timeout=25)
  (out/'fs-checks.log').write_bytes(r.stdout+r.stderr);print((r.stdout+r.stderr).decode('utf-8','replace'))
  r.check_returncode()
if __name__=='__main__':main()
