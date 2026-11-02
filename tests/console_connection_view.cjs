const assert = require('node:assert/strict');
require('../tools/console/ui/connection.js');
const {view,protocol} = globalThis.ConnectionUI;
assert.equal(protocol('lobby'),'upstream');
assert.equal(protocol('off'),'off');
for (const launch of ['play','online']) {
  const yuzu=view({state:'running',mode:launch,connection:'yuzu'},null,{mode:'off'});
  assert.equal(yuzu.gate,false);assert.equal(yuzu.roomControls,true);
  const upstream=view({state:'running',mode:launch,connection:'upstream'},null,{mode:'yuzu'});
  assert.equal(upstream.gate,false);assert.equal(upstream.roomControls,false);
  assert.equal(upstream.current,'upstream');
}
assert.equal(view({state:'running',mode:'smoke',connection:'off'},null,{mode:'yuzu'}).roomControls,false);
assert.equal(view({state:'running',mode:'play',connection:'yuzu'},{connection:'yuzu',started:true},{mode:'lobby'}).gate,false);
assert.equal(view({state:'running',mode:'play',connection:'yuzu',restarting:true},null,{mode:'yuzu'}).restart,false);
assert.equal(view({state:'idle'},null,{mode:'off'}).current,'off');
console.log('PASS connection display follows the actual process; immediate Yuzu boot, smoke and restart policy');
