"""Build the native networking fixture through setup's staging and the real host builder.

The game/renderer glue is replaced only inside the isolated fixture directory. This
validates network source/dependency compilation and metadata, not a playable game.
"""
import contextlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / 'tests'))
from scripts import network_launch, setup
from test_network_build_launch import load_builder, EVIDENCE, CLANG, DEPS


def main():
    root = Path(tempfile.mkdtemp(prefix='host-', dir=EVIDENCE))
    work = root / 'work'
    work.mkdir()
    output = io.StringIO()
    variables = {'RECOMP_CLANG': CLANG, 'SSBU_NETWORK_DEPS': str(DEPS),
                 'SDL3_ROOT': str(DEPS / 'SDL3'), 'VULKAN_HEADERS': str(DEPS / 'Vulkan-Headers')}
    # Standard stage behavior copies the current source files into this diagnostic build only.
    with contextlib.redirect_stdout(output):
        setup.step_stage(SimpleNamespace(work=str(work), state={}))
    for name in ('network_protocol.c', 'network_protocol.h', 'network_abi.h', 'build_network.py'):
        assert (work / name).is_file(), name
    shutil.copy2(ROOT / 'tests' / 'network_protocol.c', work / 'network_fixture.c')
    shutil.copy2(ROOT / 'tests' / 'yuzu_backend.c', work / 'yuzu_backend.c')
    # No generated game code is available or needed for the protocol fixture.
    (work / 'gamedll.c').write_text('/* game loader omitted from the isolated network fixture */\n')
    sources = ['network_protocol.c', 'nnnet.c', 'nnldn.c', 'rb_net.c',
               'yuzu_net.c', 'yuzu_ldn.c', 'yuzu_socket.c', 'yuzu_ui.c', 'console_bridge.c',
               'network_fixture.c']
    variables['RECOMP_EXE'] = 'network_host_fixture.exe'
    with patch.dict(os.environ, variables), patch.object(sys, 'argv', ['build.py', 'host', '--with-yuzu']):
        builder = load_builder(work / 'build.py', ('--with-yuzu',), variables)
        builder.HOST_SOURCES = sources
        builder.HAVE_IMGUI = True
        builder.CXX_SOURCES = []
        # Keep the builder's real game import-library generation, SDL3 and linker flags.
        sys.path.insert(0, str(work))
        try:
            with contextlib.redirect_stdout(output):
                result = builder.cmd_link(host_only=True)
            (root / 'build.log').write_text(output.getvalue(), encoding='utf-8')
            assert result == 0, output.getvalue()[-5000:]
            original_compile = builder.compile_host
            compiled = []
            def track(job, host_flags=None):
                compiled.append(job[0])
                return original_compile(job, host_flags)
            builder.compile_host = track
            with contextlib.redirect_stdout(output):
                assert builder.cmd_link(host_only=True) == 0
            assert not compiled, compiled
        finally:
            sys.path.remove(str(work))
    exe = work / 'network_host_fixture.exe'
    feature = network_launch.host_features(exe)
    assert feature['yuzu_network'] is True and feature['network_abi'] == 1
    assert feature['resource_mods'] is False and feature['latency_slider'] is False
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith(('SSBU_', 'RECOMP_'))}
    env.update(SSBU_GRAPHICS_CONFIG=str(root / 'graphics.ini'))
    (root / 'graphics.ini').write_text('online_mode=yuzu\nrollback=on\n')
    assert network_launch.configure_network(exe, env, auto_rollback=True) == 'yuzu'
    assert env['SSBU_RB_NET'] == '0'
    result = subprocess.run([str(exe)], cwd=work, env=env, capture_output=True, timeout=20)
    (root / 'runtime.log').write_bytes(result.stdout + result.stderr)
    assert not result.returncode, (result.returncode, (result.stdout + result.stderr).decode('utf-8', 'replace'))
    (root / 'build.log').write_text(output.getvalue(), encoding='utf-8')
    evidence = {'stage': 'passed', 'native_build': 'passed', 'repeat_host_recompiled': len(compiled),
                'network_fixture_execution': 'passed', 'feature_sha256_verified': True,
                'game_loader_replaced': True, 'whole_game_built': False, 'directory': str(root)}
    (EVIDENCE / 'host-build-results.json').write_text(json.dumps(evidence, indent=2), encoding='utf-8')
    print('PASS staged host builder, real network dependencies/link, exact feature record, incremental rebuild and SDK protocol run')
    print('Evidence:', root)


if __name__ == '__main__':
    main()
