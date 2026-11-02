"""Network-only build and launch checks. No game dump, real room or user config is used."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from scripts import network_launch, run, setup, yuzu_play

EVIDENCE = ROOT / 'build' / 'network-build-20261024'
EVIDENCE.mkdir(parents=True, exist_ok=True)
CLANG = os.environ.get('RECOMP_CLANG') or str(ROOT / 'deps' / 'llvm' / 'bin' / 'clang.exe')
DEPS = Path(os.environ.get('SSBU_NETWORK_DEPS') or ROOT / 'deps')


def feature_record(exe, enabled=True):
    record = {'schema_version': 2, 'yuzu_network': enabled, 'network_abi': 1,
              'exe_sha256': hashlib.sha256(exe.read_bytes()).hexdigest()}
    exe.with_suffix('.build.json').write_text(json.dumps(record), encoding='utf-8')


def load_builder(path, args=(), env=None):
    spec = importlib.util.spec_from_file_location('network_build_fixture', path)
    module = importlib.util.module_from_spec(spec)
    variables = {'RECOMP_TARGET': '', 'RECOMP_YUZU': '1', 'IMGUI_ROOT': str(EVIDENCE / 'no-imgui')}
    variables.update(env or {})
    with patch.object(sys, 'argv', ['build.py', 'host', *args]), patch.dict(os.environ, variables):
        spec.loader.exec_module(module)
    return module


class NetworkBuildLaunch(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=EVIDENCE)
        self.addCleanup(self.temp.cleanup)
        self.build = Path(self.temp.name)
        self.work = self.build / 'work'
        self.work.mkdir()
        self.exe = self.work / 'ssbu_native.exe'
        self.exe.write_bytes(b'fixture executable')
        feature_record(self.exe)
        for name in ('ssbu_game_eh_o1.dll', 'SDL3.dll', 'ssbu_glslang.dll'):
            (self.work / name).write_bytes(b'fixture')
        (self.build / 'config').mkdir()
        self.config = self.build / 'config' / 'graphics.ini'
        self.config.write_text('online_mode=yuzu\nrollback=on\n', encoding='utf-8')
        (self.build / 'state.json').write_text(json.dumps({'game_version': '13.0.5', 'dump': {'romfs': str(self.build / 'romfs')}}), encoding='utf-8')

    def test_saved_yuzu_blocks_auto_and_explicit_network_rollback(self):
        for requested in (None, 'net', '1'):
            env = {} if requested is None else {'SSBU_RB_NET': requested}
            self.assertEqual(network_launch.configure_network(self.exe, env, auto_rollback=True), 'yuzu')
            self.assertEqual(env['SSBU_RB_NET'], '0')
        self.assertIn('rollback=on', self.config.read_text())

    def test_explicit_protocol_override_wins_over_saved_mode(self):
        env = {'ssbu_online': 'lobby'}
        self.assertEqual(network_launch.configure_network(self.exe, env, auto_rollback=True), 'upstream')
        self.assertEqual(env['SSBU_RB_NET'], 'net')
        self.assertNotIn('ssbu_online', env)
        for value in ('', 'off', 'false'):
            env = {'SSBU_ONLINE': value, 'SSBU_RB_NET': 'net'}
            self.assertEqual(network_launch.configure_network(self.exe, env, auto_rollback=True), 'off')
            self.assertEqual(env['SSBU_RB_NET'], '0')

    def test_manual_rollback_off_and_local_loop_are_preserved(self):
        env = {'SSBU_ONLINE': 'lobby', 'SSBU_RB_NET': '0'}
        network_launch.configure_network(self.exe, env, auto_rollback=True)
        self.assertEqual(env['SSBU_RB_NET'], '0')
        env = {'SSBU_RB_NET': 'loop,4'}
        network_launch.configure_network(self.exe, env, auto_rollback=True)
        self.assertEqual(env['SSBU_RB_NET'], 'loop,4')

    def test_bom_case_and_legacy_settings(self):
        for text, expected in [('\ufeff online_mode = YuZu # comment\n', 'yuzu'),
                               ('online_server=off\n', 'off'), ('online_server=example:39500\n', 'upstream'),
                               ('online_mode=typo\n', 'off')]:
            self.config.write_text(text, encoding='utf-8')
            self.assertEqual(network_launch.configure_network(self.exe, {}), expected)

    def test_graphics_path_precedence_and_relative_paths(self):
        alt = self.work / 'alt'
        alt.mkdir()
        (alt / 'graphics.ini').write_text('online_mode=off\n', encoding='utf-8')
        env = {'SSBU_CONFIG': 'alt/controllers.ini'}
        self.assertEqual(network_launch.configure_network(self.exe, env), 'off')
        env['SSBU_GRAPHICS_CONFIG'] = str(self.config)
        self.assertEqual(network_launch.configure_network(self.exe, env), 'yuzu')
        env = {'SSBU_CONFIG': 'controllers.ini'}
        (self.work / 'graphics.ini').write_text('online_mode=lan\n', encoding='utf-8')
        self.assertEqual(network_launch.configure_network(self.exe, env), 'upstream')

    def test_yuzu_rejects_upstream_missing_or_replaced_feature_records(self):
        for operation in ('missing', 'disabled', 'replaced', 'abi'):
            self.exe.write_bytes(b'fixture executable')
            feature_record(self.exe)
            stamp = self.exe.with_suffix('.build.json')
            if operation == 'missing': stamp.unlink()
            elif operation == 'disabled': feature_record(self.exe, False)
            elif operation == 'replaced': self.exe.write_bytes(b'replaced host')
            else:
                data = json.loads(stamp.read_text()); data['network_abi'] = 2; stamp.write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                network_launch.configure_network(self.exe, {}, require_yuzu=True)

    def test_unmarked_upstream_host_keeps_upstream_and_off_modes(self):
        self.exe.with_suffix('.build.json').unlink()
        for mode in ('lobby', 'off'):
            env = {'SSBU_ONLINE': mode}
            self.assertEqual(network_launch.configure_network(self.exe, env, auto_rollback=True), 'upstream' if mode == 'lobby' else 'off')
        self.config.unlink()
        self.assertEqual(network_launch.configure_network(self.exe, {}), 'upstream')

    def test_normal_launcher_passes_final_mode_and_game_version(self):
        args = ['run.py', '--build', str(self.build), '--play', '--nickname', 'Player', '--env', 'SSBU_RB_NET=net']
        for mode in ('yuzu', 'lobby', 'off'):
            with patch.object(run, 'REPO', str(self.build)), patch.object(sys, 'argv', [*args, '--env', 'SSBU_ONLINE=' + mode]), patch.object(run.subprocess, 'run') as spawn, contextlib.redirect_stdout(io.StringIO()):
                spawn.return_value = SimpleNamespace(returncode=0)
                self.assertEqual(run.main(), 0)
                child = spawn.call_args
                self.assertEqual(child.kwargs['env']['SSBU_RB_NET'], 'net' if mode == 'lobby' else '0')
                self.assertEqual(child.kwargs['env']['SSBU_ONLINE'], mode)
                self.assertIn('13.0.5', child.args[0][1])

    def test_copied_test_executable_keeps_exact_feature_record(self):
        args = ['run.py', '--build', str(self.build), '--userdata', str(self.build / 'saves'), '--nickname', 'Player', '--exe', 'test_copy.exe']
        with patch.object(sys, 'argv', args), patch.object(run.subprocess, 'run') as spawn, contextlib.redirect_stdout(io.StringIO()):
            spawn.return_value = SimpleNamespace(returncode=0)
            self.assertEqual(run.main(), 0)
        self.assertEqual(network_launch.host_features(self.work / 'test_copy.exe')['yuzu_network'], True)

    def test_dedicated_launcher_forces_yuzu_without_changing_saved_settings(self):
        self.config.write_text('online_mode=lobby\nrollback=on\n', encoding='utf-8')
        with patch.object(yuzu_play, 'ROOT', self.build), patch.object(sys, 'argv', ['yuzu_play.py', '--nickname', 'Player']):
            # Use a child build layout, as the dedicated entry always does.
            nested = self.build / 'build'; nested.mkdir()
            for name in ('state.json',): shutil.copy2(self.build / name, nested / name)
            shutil.copytree(self.work, nested / 'work')
            shutil.copytree(self.build / 'config', nested / 'config')
            with patch.object(yuzu_play.subprocess, 'Popen') as spawn, contextlib.redirect_stdout(io.StringIO()):
                spawn.return_value = SimpleNamespace(pid=123)
                self.assertEqual(yuzu_play.main(), 0)
                env = spawn.call_args.kwargs['env']
                self.assertEqual(env['SSBU_ONLINE'], 'yuzu')
                self.assertEqual(env['SSBU_RB_NET'], '0')
                self.assertIn('13.0.5', spawn.call_args.args[0][1])
                self.assertEqual(spawn.call_args.kwargs['env']['SSBU_GAME_VERSION'], '13.0.5')
        self.assertEqual(self.config.read_text(), 'online_mode=lobby\nrollback=on\n')

    def test_source_plan_keeps_upstream_dll_split_and_yuzu_flag(self):
        for args, enabled in [((), True), (('--without-yuzu',), False)]:
            builder = load_builder(ROOT / 'tools' / 'lifter' / 'build.py', args)
            self.assertEqual(builder.YUZU_NETWORK, enabled)
            self.assertEqual('network_protocol.c' in builder.HOST_SOURCES, enabled)
            self.assertIn('-DSSBU_ENABLE_YUZU=' + str(int(enabled)), builder.HOST_FLAGS)
            self.assertIn('-DSSBU_ENABLE_YUZU=' + str(int(enabled)), builder.CXX_FLAGS)
            for source in ('sdkdata.c', 'nro_registry.c', 'sdklift_gen.c'):
                self.assertNotIn(source, builder.HOST_SOURCES)
                self.assertIn(source, builder.GAME_DATA_SOURCES)
            self.assertIn('nnpad_gca.c', builder.HOST_SOURCES)
            self.assertEqual(len(builder.HOST_SOURCES), len(set(builder.HOST_SOURCES)))

    def test_prebuilt_runtime_is_rejected_before_any_copy(self):
        cx = SimpleNamespace(a=SimpleNamespace(with_yuzu=True), build=str(self.build), runtime_dir=None)
        with patch.object(setup.shutil, 'copy2') as copy, self.assertRaisesRegex(setup.Fail, 'Yuzu networking'):
            setup.load_runtime(cx, self.build / 'nonexistent.zip')
        copy.assert_not_called()
        self.assertIsNone(cx.runtime_dir)

    def test_setup_relinks_on_yuzu_build_choice_change(self):
        state_path = self.build / 'state.json'
        state_path.write_text(json.dumps({'done': {step: {'secs': 0} for step in setup.STEPS}, 'features': {'resource_mods': False, 'latency_slider': False, 'yuzu_network': False}}))
        for name in ('SDL3.dll', 'ssbu_game_eh_o1.dll'):
            (self.work / name).write_bytes(b'fixture')
        linked = []
        def link(cx):
            linked.append(cx.a.with_yuzu)
            cx.state.setdefault('features', {})['yuzu_network'] = cx.a.with_yuzu
        runners = {step: (lambda cx: None) for step in setup.STEPS}; runners['link'] = link
        with patch.object(setup, 'RUNNERS', runners), patch.object(setup, 'say'), patch.object(setup, 'how_to_run'):
            for flag in ('--with-yuzu', '--with-yuzu', '--without-yuzu'):
                with patch.object(sys, 'argv', ['setup.py', '--build', str(self.build), flag]):
                    self.assertEqual(setup.main(), 0)
        self.assertEqual(linked, [True, False])

    def test_setup_build_env_keeps_network_cache_in_source_deps(self):
        cx = SimpleNamespace(linux=False, mac=False, a=SimpleNamespace(with_yuzu=True, no_inline=False), env=lambda **kwargs: kwargs)
        with patch.dict(os.environ, {'SSBU_NETWORK_DEPS': str(DEPS)}):
            env = setup.build_env(cx)
        self.assertEqual(Path(env['SSBU_NETWORK_DEPS']), DEPS.resolve())
        self.assertEqual(env['RECOMP_YUZU'], '1')


if __name__ == '__main__':
    unittest.main(verbosity=2)
