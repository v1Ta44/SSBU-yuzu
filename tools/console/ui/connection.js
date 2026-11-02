/* Shared view decisions; the running process always wins over saved next-start settings. */
(() => {
  const labels = {yuzu:'Yuzu 房间',off:'关闭联机',lobby:'上游大厅',lan:'上游局域网',host:'上游房间码 · 创建',join:'上游房间码 · 加入',upstream:'上游联机'};
  function protocol(mode) { return mode === 'yuzu' ? 'yuzu' : mode === 'off' ? 'off' : 'upstream'; }
  function view(game, net, saved) {
    const running = game?.state === 'running';
    const current = running ? (net?.connection || game.connection || protocol(saved?.mode || 'yuzu')) : protocol(saved?.mode || 'yuzu');
    const yuzu = current === 'yuzu';
    return {running,current,yuzu,roomControls:yuzu && game?.mode !== 'smoke',
      gate:false,
      restart:running && game?.mode !== 'smoke' && !game?.restarting};
  }
  globalThis.ConnectionUI = {labels,protocol,view};
})();
