import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const source = await readFile(
  new URL('../../apps/desktop-ui/src/features/rig/RigWorkspace.tsx', import.meta.url),
  'utf8',
);

test('Rig bone editor exposes revision-bound scale controls and an honest skeleton debug view', () => {
  assert.match(source, /Scale X ×/);
  assert.match(source, /Scale Y ×/);
  assert.match(source, /scaleXPpm:\s*Math\.round\(value\s*\*\s*1_000_000\)/);
  assert.match(source, /scaleYPpm:\s*Math\.round\(value\s*\*\s*1_000_000\)/);
  assert.match(source, /Math\.abs\(value\)\s*>=\s*0\.001/);
  assert.match(source, /rotationRad:\s*base\.rotationRad\s*\+\s*localRotation/);
  assert.match(source, /scaleX:\s*base\.scaleX\s*\*\s*localScaleX/);
  assert.match(source, /rig\.setSlot/);
  assert.match(source, /Slot 绑定与绘制顺序/);
  assert.match(source, /expectedRevision:\s*rig\.revision,[\s\S]*slotId:\s*value\.slotId/);
  assert.equal((source.match(/tool === 'slots'/g) ?? []).length, 1);
  assert.match(source, /rig\.reparentBone/);
  assert.match(source, /rig\.setPivot/);
  assert.match(source, /rig\.setSocket/);
  assert.match(source, /骨骼点\/线位置传播当前父级平移、旋转与缩放/);
  assert.match(source, /背景角色图保持静态/);
  assert.doesNotMatch(source, />17 层 slot\/pivot\/mesh\/weight/);
});
