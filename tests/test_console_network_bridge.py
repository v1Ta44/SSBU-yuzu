"""Compile and run the real Console pipe with the networking adapters, without game/GPU.
Use --clang and --deps to reuse an existing compiler and ENet/Zstandard cache.
The resulting executable also serves the explicit Rust restart integration test.
"""
import argparse
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools' / 'lifter'))
import build_network


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--clang', default=os.environ.get('RECOMP_CLANG') or str(ROOT / 'deps/llvm/bin/clang.exe'))
    parser.add_argument('--deps')
    args = parser.parse_args()
    if args.deps:
        os.environ['SSBU_NETWORK_DEPS'] = str(Path(args.deps).resolve())
    out = ROOT / 'build/network-ui-20261024'
    out.mkdir(parents=True, exist_ok=True)
    includes, objects = build_network.prepare(args.clang, ROOT / 'build/work')
    names = ('yuzu_net', 'yuzu_ldn', 'yuzu_socket', 'network_protocol',
             'nnnet', 'nnldn', 'rb_net', 'console_bridge')
    sources = [ROOT / 'tests/console_network_bridge.c'] + [ROOT / 'runtime' / (n + '.c') for n in names]
    exe = out / 'console_network_bridge.exe'
    result = subprocess.run([args.clang, '-O1', '-w', '-DSSBU_ENABLE_YUZU=1', '-I', str(ROOT / 'runtime'),
                             *includes, *map(str, sources), *objects, '-lws2_32', '-lwinmm',
                             '-ladvapi32', '-luser32', '-o', str(exe)], capture_output=True, timeout=180)
    (out / 'bridge-build.log').write_bytes(result.stdout + result.stderr)
    result.check_returncode()
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith(('SSBU_', 'RECOMP_'))}
    result = subprocess.run([str(exe)], cwd=out, env=env, capture_output=True, timeout=40)
    (out / 'bridge-run.log').write_bytes(result.stdout + result.stderr)
    result.check_returncode()
    print(result.stdout.decode('utf-8', 'replace').strip())


if __name__ == '__main__':
    main()
