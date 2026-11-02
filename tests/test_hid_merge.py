"""Compile and exercise real Npad writing, final-state tracing and menu input with virtual devices.
No game, renderer, physical controller, user configuration or desktop focus is used.
"""
import argparse
import csv
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--clang', default=os.environ.get('RECOMP_CLANG') or str(ROOT / 'deps/llvm/bin/clang.exe'))
    args = parser.parse_args()
    out = ROOT / 'build/focus-input-20261025'
    out.mkdir(parents=True, exist_ok=True)
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith(('SSBU_', 'RECOMP_'))}
    results = []
    for source in ['hid_merge_test', 'input_trace_test']:
        exe = out / (source + '.exe')
        result = subprocess.run([args.clang, '-O1', '-w', '-I', str(ROOT / 'runtime'),
                                 str(ROOT / 'tests' / (source + '.c')), '-luser32', '-o', str(exe)],
                                capture_output=True, timeout=120)
        (out / (source + '-build.log')).write_bytes(result.stdout + result.stderr)
        result.check_returncode()
        cases = [('trace-on', str(out / (source + '.csv'))), ('trace-off', '')]
        if source == 'hid_merge_test':
            cases.append(('xinput-selftest', '--xinput-selftest'))
        for case, argument in cases:
            result = subprocess.run([str(exe), argument], cwd=out, env=env, capture_output=True, timeout=30)
            (out / (source + '-' + case + '.log')).write_bytes(result.stdout + result.stderr)
            result.check_returncode()
            results.append({'test': source, 'case': case, 'passed': True})
            print('PASS real input', source, case)
        rows = list(csv.DictReader((out / (source + '.csv')).read_text(encoding='utf-8').splitlines()[1:]))
        if source == 'hid_merge_test':
            assert len(rows) == 5, rows  # unchanged polls deduplicate
            assert [r['buttons_hex'] for r in rows] == ['1', '3', '0', '3', '0']
            assert rows[0]['ry'] == '-480' and rows[1]['ry'] == '900'
            assert rows[2]['attributes'] == '3' and rows[-1]['attributes'] == '0'
            assert all(r['player'] == '2' for r in rows)
        else:
            assert len(rows) == 4 and [r['player'] for r in rows] == ['2', '2', '8', '2']
            assert rows[1]['ry'] == '900' and rows[-1]['attributes'] == '0'
    (out / 'input-results.json').write_text(json.dumps(results, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
