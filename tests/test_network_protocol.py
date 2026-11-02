"""Validate Yuzu as an online_mode option through real native SDK adapters.

Windows; needs the same compiler/ENet/Zstandard sources as test_yuzu_backend.py.
All configuration, logs and executables stay under build/network-protocol-20261024.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools' / 'lifter'))
import build_network


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--clang', default=str(ROOT / 'deps' / 'llvm' / 'bin' / 'clang.exe'))
    parser.add_argument('--deps', help='read existing ENet/Zstandard source cache here')
    args = parser.parse_args()
    if args.deps:
        os.environ['SSBU_NETWORK_DEPS'] = str(Path(args.deps).resolve())
    work = ROOT / 'build' / 'network-protocol-20261024'
    work.mkdir(parents=True, exist_ok=True)
    includes, objects = build_network.prepare(args.clang, ROOT / 'build' / 'work')
    names = ('yuzu_net', 'yuzu_ldn', 'yuzu_socket', 'network_protocol', 'nnldn', 'nnnet', 'rb_net')
    sources = [ROOT / 'tests' / 'network_protocol.c'] + [ROOT / 'runtime' / (n + '.c') for n in names]
    exe = work / 'network_protocol_test.exe'
    subprocess.run([args.clang, '-O1', '-w', '-DSSBU_ENABLE_YUZU=1', '-I', str(ROOT / 'runtime'),
                    *includes, *map(str, sources), *objects, '-lws2_32', '-lwinmm', '-ladvapi32', '-luser32',
                    '-o', str(exe)], check=True)
    base_env = {k: v for k, v in os.environ.items() if not k.upper().startswith('SSBU_')}
    cfg = work / 'graphics.ini'
    results = []

    def run(name, argument, env):
        result = subprocess.run([str(exe)] + ([argument] if argument else []), env=env,
                                cwd=work, capture_output=True, timeout=20)
        (work / (name + '.log')).write_bytes(result.stdout + result.stderr)
        if result.returncode:
            raise AssertionError((name, result.returncode, (result.stdout + result.stderr).decode('utf-8', 'replace')))
        results.append(name)
        return result.stdout

    cases = [('missing', '', 2), ('bom', '\ufeffonline_mode=yuzu\n', 2),
             ('legacy-off', 'online_server=off\n', 0), ('legacy-server', 'online_server=127.0.0.1:39500\n', 1),
             ('explicit-over-legacy', 'online_server=off\nonline_mode=yuzu\n', 2), ('comments', '; online_mode = off\n', 2),
             ('yuzu', 'online_mode = yuzu\n', 2), ('off', 'online_mode=off\n', 0),
             ('zero', 'online_mode = 0\n', 0), ('false', 'online_mode = false\n', 0),
             ('lan', 'online_mode=lan\n', 1), ('host', 'online_mode = host\n', 1),
             ('join', 'online_mode=join:ABCDE-FGHIJ\n', 1), ('lobby', 'online_mode=lobby\n', 1),
             ('server', 'online_mode=localhost:39500\n', 1), ('on', 'online_mode=on\n', 1),
             ('whitespace', '  online_mode \t= YuZu  # comment\n', 2),
             ('similar-key', 'online_mode_other=off\nonline_mode=yuzu\n', 2),
             ('invalid', 'online_mode=typo\n', 0)]
    for name, text, expected in cases:
        cfg.write_text(text, encoding='utf-8')
        env = dict(base_env, SSBU_GRAPHICS_CONFIG=str(cfg))
        assert run('mode-' + name, '--mode', env).strip() == str(expected).encode(), name
    cfg.write_text('online_mode=yuzu\n', encoding='utf-8')
    for value, expected in [('', 0), ('off', 0), ('yuzu', 2), ('lobby', 1), ('127.0.0.1:39500', 1)]:
        env = dict(base_env, SSBU_GRAPHICS_CONFIG=str(cfg), SSBU_ONLINE=value)
        assert run('override-' + value.replace(':', '-'), '--mode', env).strip() == str(expected).encode()
    # Controller sibling and relative filenames must match the graphics settings path.
    env = dict(base_env, SSBU_CONFIG=str(work / 'controllers.ini'))
    assert run('controller-sibling', '--mode', env).strip() == b'2'
    env = dict(base_env, SSBU_CONFIG='controllers.ini')
    assert run('controller-relative', '--mode', env).strip() == b'2'
    cfg.write_text('online_mode=off\n', encoding='utf-8')
    env = dict(base_env, SSBU_CONFIG=str(work / 'controllers.ini'), SSBU_GRAPHICS_CONFIG=str(cfg))
    assert run('graphics-path-priority', '--mode', env).strip() == b'0'
    for mode in ('yuzu', 'off', 'lobby'):
        cfg.write_text('online_mode=' + mode + '\n', encoding='utf-8')
        run('selection-' + mode, '--select', dict(base_env, SSBU_GRAPHICS_CONFIG=str(cfg)))
    cfg.write_text('online_mode=yuzu\n', encoding='utf-8')
    run('restart-only', '--cache', dict(base_env, SSBU_GRAPHICS_CONFIG=str(cfg)))
    run('local-loop', '--loop', dict(base_env, SSBU_GRAPHICS_CONFIG=str(cfg)))
    run('sdk-integration', None, dict(base_env, SSBU_GRAPHICS_CONFIG=str(cfg)))
    (work / 'results.json').write_text(json.dumps({'passed': len(results), 'cases': results}, indent=2), encoding='utf-8')
    print(f'PASS {len(results)} native protocol/configuration/SDK integration cases; logs: {work}')


if __name__ == '__main__':
    main()
