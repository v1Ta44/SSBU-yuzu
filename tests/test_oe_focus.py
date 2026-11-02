"""Exercise focus notifications while the guest waits instead of polling, using real runtime events.
Use --isolated for real events/watcher tests with game/renderer boundaries stubbed.
The original full-host path requires an existing game build. Neither path starts a game session or GPU.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def existing_host_main():
    state = json.loads((ROOT / 'build/state.json').read_text())
    work = ROOT / 'build/work'
    os.environ.update(RECOMP_CLANG=state['tools']['clang'], SDL3_ROOT=state['tools']['sdl3'],
                      VULKAN_HEADERS=state['tools']['vulkan'])
    sys.path.insert(0, str(work))
    spec = importlib.util.spec_from_file_location('focus_build', work / 'build.py')
    build = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(build)
    outdir = work / 'obj_host/oe_focus_test'
    outdir.mkdir(parents=True, exist_ok=True)
    testobj, bootobj = outdir / 'oe_focus.obj', outdir / 'boot.obj'
    for source, obj, extra in [(ROOT / 'tests/oe_focus_test.c', testobj, []),
                               (work / 'boot.c', bootobj, ['-Dmain=focus_unused_game_main'])]:
        result = subprocess.run([build.CLANG, '-c', *build.HOST_FLAGS, *extra, str(source), '-o', str(obj)],
                                capture_output=True, text=True, errors="replace")
        if result.returncode:
            print(result.stdout + result.stderr)
            return result.returncode
    network = []
    if (work / "build_network.py").is_file():
        import build_network
        _, network = build_network.prepare(build.CLANG, str(work))
    objdir = work / 'obj_host/ssbu_native'
    objects = [str(objdir / (Path(n).stem + '.obj')) for n in build.HOST_SOURCES + ['gamedll.c']
               if n not in ('boot.c', 'nnmisc.c', 'os_threads.c')]
    exe = work / 'oe_focus_test.exe'
    result = subprocess.run([build.CLANG, str(testobj), str(bootobj), *objects, *network,
                             str(work / 'ssbu_game_eh_o1.lib'), str(Path(build.SDL_DIR) / 'SDL3.lib'),
                             *build.LIBS, *build.debug_link_flags(str(exe)), '-o', str(exe)],
                            capture_output=True, text=True, errors="replace")
    if result.returncode:
        print(result.stdout + result.stderr)
        return result.returncode
    subprocess.run([str(exe)], cwd=work, check=True, timeout=20)
    subprocess.run([str(exe), '--guest-bootstrap'], cwd=work, check=True, timeout=20)
    subprocess.run([str(exe), '--disabled'], cwd=work, check=True, timeout=20)
    subprocess.run([str(exe), '--default'], cwd=work, check=True, timeout=20)
    return 0


def isolated_main(clang):
    out = ROOT / 'build/focus-input-20261025'
    out.mkdir(parents=True, exist_ok=True)
    exe = out / 'oe_focus_test.exe'
    command = [clang, '-O1', '-w', '-I', str(ROOT / 'runtime'),
               str(ROOT / 'tests/oe_focus_standalone.c'), '-ladvapi32', '-luser32', '-o', str(exe)]
    build = subprocess.run(command, capture_output=True, timeout=120)
    (out / 'focus-build.log').write_bytes(build.stdout + build.stderr)
    build.check_returncode()
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith(('SSBU_', 'RECOMP_'))}
    results = []
    for case in ['', '--guest-bootstrap', '--disabled', '--default', '--exit-nested', '--exit-suspended', '--exit-background']:
        run = subprocess.run([str(exe)] + ([case] if case else []), cwd=out, env=env, capture_output=True, timeout=20)
        (out / ('focus' + case + '.log')).write_bytes(run.stdout + run.stderr)
        run.check_returncode()
        text = run.stdout.decode('utf-8', 'replace')
        assert 'PASS ' in text, (case, text)
        results.append({'case': case or 'explicit-upstream', 'passed': True, 'checks': text.count('PASS ')})
        print('PASS isolated focus/exit', case or 'explicit-upstream')
    (out / 'focus-results.json').write_text(json.dumps(results, indent=2) + '\n', encoding='utf-8')
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--isolated', action='store_true', help='use real nn::oe/nn::os with no game build')
    parser.add_argument('--clang', default=os.environ.get('RECOMP_CLANG') or str(ROOT / 'deps/llvm/bin/clang.exe'))
    args = parser.parse_args()
    return isolated_main(args.clang) if args.isolated else existing_host_main()


if __name__ == '__main__':
    sys.exit(main())
