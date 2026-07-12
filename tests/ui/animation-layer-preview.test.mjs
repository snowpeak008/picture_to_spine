import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('animation preview is driven by CAS LayerSet attachments and sampled slot/bone state', async () => {
  const [host, preview, workspace] = await Promise.all([
    readFile('apps/desktop/src-tauri/src/ipc_host.rs', 'utf8'),
    readFile('apps/desktop-ui/src/features/animation/PixiRigPreview.tsx', 'utf8'),
    readFile('apps/desktop-ui/src/features/animation/AnimationWorkspace.tsx', 'utf8'),
  ]);
  for (const required of ['attachmentPreview', 'safePngDataUrl', 'data:image/png;base64,', 'rigid_preview_bone', 'render_safe_preview_png', 'cas_get(cas, &layer.attachment_sha256']) {
    assert.ok(host.includes(required), required);
  }
  for (const required of ['attachmentPreview.attachments', 'new Sprite(texture)', "channel === 'bone-translate'", "channel === 'bone-rotate'", "channel === 'bone-scale'", "channel === 'slot-color'", "channel === 'draw-order'", 'sprite.position.set', 'sprite.rotation', 'sprite.scale.set', 'sprite.visible', 'sprite.zIndex']) {
    assert.ok(preview.includes(required), required);
  }
  assert.equal(preview.includes('previewDataUrl'), false);
  assert.ok(preview.includes('attachment.drawKey + Math.round'));
  assert.ok(workspace.includes('Mesh/deform 和多骨蒙皮未模拟'));
  assert.ok(workspace.includes('unsupportedLayerIds'));
  assert.equal(host.includes('safePngPath'), false);
  assert.equal(host.includes('file://'), false);
  assert.equal(host.includes('"multiBoneSkinningApplied": true'), false);
  assert.equal(host.includes('"meshDeformationApplied": true'), false);
});
