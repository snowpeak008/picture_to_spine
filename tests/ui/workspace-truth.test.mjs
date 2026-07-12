import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('export UI consumes Native preflight and never hard-codes produced files as READY',async()=>{
  const source=await readFile('apps/desktop-ui/src/features/export/ExportWorkspace.tsx','utf8');
  for(const required of ['export.preflight','export.chooseRootAndCommit','data.outputs','data.history','EXPORTED_UNVERIFIED'])assert.ok(source.includes(required),required);
  assert.equal(source.includes("const outputs=["),false);
  assert.equal(source.includes("'READY'"),false);
  assert.equal(source.includes('内置合同</dt><dd className="ok">VERIFIED'),false);
});

test('animation UI edits the authoritative clip and keeps gameplay markers separate',async()=>{
  const [source,host,service]=await Promise.all([
    readFile('apps/desktop-ui/src/features/animation/AnimationWorkspace.tsx','utf8'),
    readFile('apps/desktop/src-tauri/src/ipc_host.rs','utf8'),
    readFile('crates/application/src/animation/set_service.rs','utf8'),
  ]);
  for(const required of ['animation.track.put','animation.poseMarker.set','animation.hitMarker.set','expectedAnimationRevision','保存命中帧','contactPhase','gameplayMarkers','新增可编辑 Track','requiresHitFrame','approvedImages!==requiredImages'])assert.ok(source.includes(required),required);
  assert.ok(host.includes('IpcMethod::AnimationHitMarkerSet'));
  assert.ok(service.includes('set_hit_frame_marker'));
  assert.equal(source.includes("channel:'hit-frame'"),false);
  assert.equal(source.includes('Math.sin(Date.now'),false);
});

test('animation preview renders bounded CAS attachments without claiming mesh skinning',async()=>{
  const [ui,host]=await Promise.all([
    readFile('apps/desktop-ui/src/features/animation/PixiRigPreview.tsx','utf8'),
    readFile('apps/desktop/src-tauri/src/ipc_host.rs','utf8'),
  ]);
  for(const required of ['attachmentPreview','safePngDataUrl','computeBoneWorlds','slot-color','draw-order'])assert.ok(ui.includes(required),required);
  for(const required of ['ANIMATION_PREVIEW_DATA_URL_BUDGET_BYTES','bounded_animation_preview_data_url','RIGID_SINGLE_BONE_FULL_CANVAS_SPRITES','multiBoneSkinningApplied'])assert.ok(host.includes(required),required);
  assert.ok(ui.includes('meshDeformationApplied: false'));
  assert.ok(ui.includes('multiBoneSkinningApplied: false'));
  assert.equal(ui.includes('safePreviewDataUrl'),false);
});

test('key-pose approval is locked behind the native bounded preview grant',async()=>{
  const [source,host,service]=await Promise.all([
    readFile('apps/desktop-ui/src/features/motion/MotionWorkspace.tsx','utf8'),
    readFile('apps/desktop/src-tauri/src/ipc_host.rs','utf8'),
    readFile('crates/application/src/motion/content_service.rs','utf8'),
  ]);
  for(const required of ['motion.keyPose.preview','motion.keyPose.alignment.set','expectedBindingRevision','groundYMilliPx','scalePpm','保存对齐并重新审核','safePreviewDataUrl','reviewToken','reviewedCurrentBinding','审批已锁定：必须先查看当前绑定的真实预览'])assert.ok(source.includes(required),required);
  assert.ok(host.includes('IpcMethod::MotionKeyPoseAlignmentSet'));
  assert.ok(service.includes('set_key_pose_alignment'));
  assert.ok(source.includes("reviewToken: review.reviewToken"));
  assert.ok(source.includes('disabled={busy || !reviewedCurrentBinding}'));
  assert.equal(source.includes("run('motion.keyPose.approve', { bindingId: binding.bindingId })"),false);
});

test('master approval covers the complete candidate and requires a bounded preview grant',async()=>{
  const [ui,host,domain]=await Promise.all([
    readFile('apps/desktop-ui/src/features/m03/master/MasterReview.tsx','utf8'),
    readFile('apps/desktop/src-tauri/src/ipc_host.rs','utf8'),
    readFile('crates/domain/src/master/master.rs','utf8'),
  ]);
  for(const required of ['master.preview','safePreviewDataUrl','reviewToken','approvalPayloadSha256'])assert.ok(ui.includes(required),required);
  assert.ok(host.includes('consume_master_review('));
  assert.ok(host.includes('master.approval_payload_sha256()?'));
  for(const boundField of ['style_spec','source_sha256','candidate_revision','supersedes'])assert.ok(domain.includes(boundField),boundField);
  assert.equal(host.includes('"approve-master",\n                    &master.source_sha256'),false);
});

test('layer workspace accepts a purpose-bound manual PNG replacement and uses real Rig diagnostics',async()=>{
  const [layerUi,host,rigUi]=await Promise.all([
    readFile('apps/desktop-ui/src/features/layers/LayerWorkspace.tsx','utf8'),
    readFile('apps/desktop/src-tauri/src/ipc_host.rs','utf8'),
    readFile('apps/desktop-ui/src/features/rig/RigWorkspace.tsx','utf8'),
  ]);
  for(const required of ['layers.replacement.chooseAndPreflight','layers.replacement.promote','全画布透明 PNG'])assert.ok(layerUi.includes(required),required);
  assert.ok(host.includes('normalize_manual_layer_png'));
  assert.ok(host.includes('render_updated_layer_attachment_png'));
  assert.ok(host.includes('PendingImportPurpose::LayerReplacement'));
  assert.ok(host.includes('diagnose_rig_candidate'));
  assert.ok(rigUi.includes('diagnostics.completed'));
  assert.equal(rigUi.includes("TemporaryRigSnapshot::new(..., vec![])"),false);
});

test('shell does not describe implemented workspaces as a visual prototype',async()=>{
  const source=await readFile('apps/desktop-ui/src/app/AppShell.tsx','utf8');
  assert.equal(source.includes('IMPLEMENTED_PARTIAL'),false);
  assert.ok(source.includes('PRODUCTION ASSIST CORE'));
  assert.ok(source.includes('Release 未授权'));
});
