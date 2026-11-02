// Fake Tauri backend for previewing the console UI in a browser.
(() => {
  const listeners = {};
  const pads = { general: { layout: 'auto', deadzone: '0.10', outer_zone: '0.98', trigger_threshold: '0.33', rumble: 'on', rumble_strength: '1.0', ignore: [] },
    players: ['auto', 'auto', 'auto', 'auto', 'auto', 'auto', 'auto', 'auto'],
    keyboard: { player: 'auto', keys: { A: 'Z', B: 'X', X: 'C', Y: 'V', L: 'Q', R: 'E', ZL: 'A', ZR: 'S', PLUS: 'ENTER', MINUS: 'BACKSPACE', LEFT: 'J', UP: 'I',
      RIGHT: 'L', DOWN: 'K', STICK_L: 'none', STICK_R: 'none', LSTICK_LEFT: 'LEFT', LSTICK_UP: 'UP', LSTICK_RIGHT: 'RIGHT', LSTICK_DOWN: 'DOWN',
      RSTICK_LEFT: 'none', RSTICK_UP: 'none', RSTICK_RIGHT: 'none', RSTICK_DOWN: 'none' } },
    devices: [{ guid: '030000005e0400000b0b000000000000', name: 'Xbox Wireless Controller', map: { ZR: 'righttrigger, button7' }, axis: {}, invert: {} }],
    exists: true, path: 'D:\ssbu\config\controllers.ini' };
  const detected = [
    { guid: '030000005e0400000b0b000000000000', name: 'Xbox Wireless Controller', type: 'Xbox One', mapped: true, player: 1, wireless: true, gyro: false, rumble: true, buttons: 16, axes: 6, hats: 1 },
    { guid: '03000000790000000600000000000000', name: 'USB Gamepad', type: 'Joystick', mapped: false, player: 2, wireless: false, gyro: false, rumble: false, buttons: 12, axes: 4, hats: 1 }];
  const emit = (ev, p) => (listeners[ev] || []).forEach(f => f({ payload: p }));
  window.__mockEmit = emit;
  const R = {
    overview: () => ({ found: true, package: true, root: 'D:\ssbu', build_id: '73a7ff4eb59a', preflight: { fatal: [], lines: ['data.arc ok'] }, sessions: [], game: { state: 'idle' } }),
    launcher_get: () => ({ romfs: '', userdata: '', romfs_default: 'romfs', smoke_seconds: '90', net: { host: '', port: 24872, nickname: '', game_name: '' }, env: [],
      fixed_seed: 'auto', minidump: '1', deterministic: '0', fight_hash: '0', zip: 'always', keep_sessions: '30', exclude_cpus: '', }),
    cover_get: () => ({ url: null }),
    controllers_get: () => JSON.parse(JSON.stringify(pads)),
    controllers_set: v => ({ path: pads.path }),
    pad_tool: () => { setTimeout(() => emit('pad-devices', { devices: detected, partial: false }), 50); return null; },
    pad_tool_stop: () => null,
    game_status: () => ({ state: 'idle' }),
    graphics_get: () => ({ values: { renderer: 'vulkan', window: 'windowed', vsync: 'auto', latency: 'normal', profile: 'custom', resolution_scale: '1' }, path: 'x', exists: true }),
  };
  // #pads,target=<guid|kb>,sel=<button>,sub=<map|slots|general>: drive the page for headless screenshots
  const q = Object.fromEntries(location.hash.slice(1).split(',').filter(Boolean).map(s => s.includes('=') ? s.split('=') : [s, '1']));
  if (q.pads) addEventListener('load', () => setTimeout(() => {
    const s = document.createElement('style'); s.textContent = '*,*::before,*::after{animation:none!important;transition:none!important}'; document.head.append(s);
    document.querySelector('.tile[data-page="pads"]').click();
    setTimeout(() => {
      if (q.sub) document.querySelector(`#page-pads .subtabs [data-sub="${q.sub}"]`).click();
      if (q.target) { const i = q.target === 'kb' ? 0 : +q.target; document.querySelectorAll('#padCards .card')[i].click(); }
      if (q.sel) setTimeout(() => document.querySelector(`#binds li[data-b="${q.sel}"] button`).click(), 50);
      if (q.hot) setTimeout(() => document.querySelector(`#binds li[data-b="${q.hot}"] button`).dispatchEvent(new PointerEvent('pointerover', { bubbles: true })), 80);
      if (q.lit) setTimeout(() => emit('pad-live', { player: 1, sources: [], buttons: q.lit.split('+'), l: [20000, 9000], r: [0, 0] }), 120);
      if (q.scroll) setTimeout(() => (document.querySelector('main').scrollTop = +q.scroll), 160);
    }, 200);
  }, 100));
  window.__TAURI__ = {
    core: { invoke: async (cmd, args) => { if (R[cmd]) return R[cmd](args); console.warn('mock: no', cmd); return null; } },
    event: { listen: async (ev, fn) => { (listeners[ev] = listeners[ev] || []).push(fn); return () => {}; } },
  };
})();
