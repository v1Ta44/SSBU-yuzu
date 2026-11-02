"""Resolve the selected connection before spawning a game; no UI or stored settings writes.

online_mode and path precedence match runtime/network_protocol.c. Feature metadata is
bound to the executable's hash so an upstream/stale binary cannot silently run Yuzu mode.
"""
import hashlib
import json
from pathlib import Path

OFF = ('', '0', 'off', 'false', 'no')
UPSTREAM = ('lobby', 'lan', 'host', 'join', '1', 'on', 'auto')


def ini_value(path, key):
    try:
        with Path(path).open(encoding='utf-8-sig', errors='replace') as stream:
            for line in stream:
                name, sep, value = line.partition('=')
                if sep and name.strip().lower() == key:
                    return value.split(';', 1)[0].split('#', 1)[0].strip()
    except FileNotFoundError:
        pass
    return None


def graphics_path(exe, env):
    """The native process resolves relative paths from its working (exe) folder."""
    work = Path(exe).resolve().parent
    if env.get('SSBU_GRAPHICS_CONFIG'):
        path = Path(env['SSBU_GRAPHICS_CONFIG'])
    elif env.get('SSBU_CONFIG'):
        path = Path(env['SSBU_CONFIG']).parent / 'graphics.ini'
    else:
        return work.parent / 'config' / 'graphics.ini'
    return path if path.is_absolute() else work / path


def host_features(exe):
    exe = Path(exe)
    stamp = exe.with_suffix('.build.json')
    try:
        features = json.loads(stamp.read_text(encoding='utf-8-sig'))
    except FileNotFoundError:
        return {}
    except (ValueError, OSError) as error:
        raise ValueError(f'Cannot read the build features for {exe.name}: {error}') from error
    if not isinstance(features, dict):
        raise ValueError(f'Invalid build features for {exe.name}; rebuild the host.')
    if features.get('yuzu_network') is True:
        with exe.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        if features.get('network_abi') != 1 or features.get('exe_sha256') != actual:
            raise ValueError(f'The Yuzu build record does not match {exe.name}; rebuild the host.')
    return features


def connection_mode(exe, env, yuzu_available):
    """Return yuzu/upstream/off; absence of Yuzu keeps legacy upstream builds usable."""
    if 'SSBU_ONLINE' in env:
        value = env['SSBU_ONLINE'].lower()
    else:
        path = graphics_path(exe, env)
        value = ini_value(path, 'online_mode')
        if value is None:
            legacy = ini_value(path, 'online_server')
            return ('off' if legacy.lower() in OFF else 'upstream') if legacy is not None else ('yuzu' if yuzu_available else 'upstream')
        value = value.lower()
    if value in OFF:
        return 'off'
    if value == 'yuzu':
        if not yuzu_available:
            raise ValueError('This executable has no verified Yuzu network build; rebuild with --with-yuzu.')
        return 'yuzu'
    if value in UPSTREAM or ':' in value:
        return 'upstream'
    return 'off'


def configure_network(exe, env, *, auto_rollback=False, require_yuzu=False):
    """Prepare the child's environment only. Saved rollback settings remain untouched."""
    # Windows environment names are case-insensitive, including --env overrides.
    for key in list(env):
        if key.upper().startswith('SSBU_') and key != key.upper():
            env[key.upper()] = env.pop(key)
    features = host_features(exe)
    if require_yuzu:
        env['SSBU_ONLINE'] = 'yuzu'
    mode = connection_mode(exe, env, features.get('yuzu_network') is True)
    setting = ini_value(graphics_path(exe, env), 'rollback')
    if mode == 'upstream':
        if auto_rollback and setting is not None and setting.lower() in ('on', '1', 'yes', 'true'):
            env.setdefault('SSBU_RB_NET', 'net')
    else:
        # Local loop diagnostics do not communicate with Yuzu peers.
        requested = env.get('SSBU_RB_NET', '')
        if not requested.lower().startswith('loop'):
            env['SSBU_RB_NET'] = '0'
    return mode
