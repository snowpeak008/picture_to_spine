import { Application, Assets, Container, Graphics, Sprite, type Texture } from 'pixi.js';
import { useEffect, useMemo, useRef, useState } from 'react';

export interface PreviewBone {
  boneId: string;
  parentId: string | null;
  rest: {
    xMilliPx: number; yMilliPx: number; rotationMilliDeg: number;
    scaleXPpm: number; scaleYPpm: number;
  };
}
export interface PreviewRig { canvas: { widthPx: number; heightPx: number }; boneTree: { bones: PreviewBone[] } }
export interface PreviewKey { tick: number; valuesMilli: number[]; curve: string; bezierMilli?: [number, number, number, number] | null }
export interface PreviewTrack { targetId: string; channel: string; keyframes: PreviewKey[] }
export interface PreviewClip { durationTicks: number; tracks: PreviewTrack[] }
export interface PreviewAttachment {
  layerId: string; layerName: string; attachmentSha256: string;
  slotId: string; boneId: string; drawKey: number;
  pivot: { xMilliPx: number; yMilliPx: number };
  visible: boolean; bindingMode: string; supported: boolean;
  meshId: string; meshPreviewApplied: false; multiBonePreviewApplied: false;
  safePngDataUrl: string;
}
export interface AttachmentPreviewProjection {
  schemaVersion: '1.0.0'; layerSetRevision: number; rigRevision: number;
  canvas: { widthPx: number; heightPx: number };
  attachments: PreviewAttachment[];
  diagnostics: {
    mode: 'RIGID_SINGLE_BONE_FULL_CANVAS_SPRITES';
    supportedAttachmentCount: number; unsupportedLayerIds: string[];
    slotColorApplied: true; drawOrderApplied: true;
    meshDeformationApplied: false; multiBoneSkinningApplied: false; note: string;
  };
}

interface Matrix2D { a: number; b: number; c: number; d: number; tx: number; ty: number }
interface SpriteBinding { attachment: PreviewAttachment; sprite: Sprite; baseScaleX: number; baseScaleY: number }

function cubic(value1: number, value2: number, t: number) {
  const inverse = 1 - t;
  return 3 * inverse * inverse * t * value1 + 3 * inverse * t * t * value2 + t * t * t;
}
function bezierAmount(amount: number, controls: [number, number, number, number] | null | undefined) {
  if (!controls) return amount;
  const [x1, y1, x2, y2] = controls.map(value => value / 1000) as [number, number, number, number];
  let low = 0; let high = 1;
  for (let index = 0; index < 12; index += 1) {
    const middle = (low + high) / 2;
    if (cubic(x1, x2, middle) < amount) low = middle; else high = middle;
  }
  return cubic(y1, y2, (low + high) / 2);
}
export function samplePreviewTrack(track: PreviewTrack | undefined, tick: number, defaults: number[]) {
  if (!track || track.keyframes.length === 0) return [...defaults];
  const nextIndex = track.keyframes.findIndex(key => key.tick >= tick);
  if (nextIndex === 0) return tick < track.keyframes[0].tick ? [...defaults] : track.keyframes[0].valuesMilli;
  if (nextIndex < 0) return track.keyframes.at(-1)!.valuesMilli;
  const left = track.keyframes[nextIndex - 1]; const right = track.keyframes[nextIndex];
  if (left.curve === 'stepped' || right.tick === left.tick) return left.valuesMilli;
  const linear = (tick - left.tick) / (right.tick - left.tick);
  const amount = left.curve === 'bezier' ? bezierAmount(linear, left.bezierMilli) : linear;
  return Array.from({ length: defaults.length }, (_, index) =>
    (left.valuesMilli[index] ?? defaults[index])
    + ((right.valuesMilli[index] ?? defaults[index]) - (left.valuesMilli[index] ?? defaults[index])) * amount);
}

function localMatrix(x: number, y: number, rotationDegrees: number, scaleX: number, scaleY: number): Matrix2D {
  const radians = rotationDegrees * Math.PI / 180; const cosine = Math.cos(radians); const sine = Math.sin(radians);
  return { a: cosine * scaleX, b: sine * scaleX, c: -sine * scaleY, d: cosine * scaleY, tx: x, ty: y };
}
function multiply(parent: Matrix2D, local: Matrix2D): Matrix2D {
  return {
    a: parent.a * local.a + parent.c * local.b, b: parent.b * local.a + parent.d * local.b,
    c: parent.a * local.c + parent.c * local.d, d: parent.b * local.c + parent.d * local.d,
    tx: parent.a * local.tx + parent.c * local.ty + parent.tx,
    ty: parent.b * local.tx + parent.d * local.ty + parent.ty,
  };
}
function apply(matrix: Matrix2D, x: number, y: number) {
  return { x: matrix.a * x + matrix.c * y + matrix.tx, y: matrix.b * x + matrix.d * y + matrix.ty };
}
function inverseApply(matrix: Matrix2D, x: number, y: number) {
  const determinant = matrix.a * matrix.d - matrix.b * matrix.c;
  if (Math.abs(determinant) < 1e-9) return { x: 0, y: 0 };
  const translatedX = x - matrix.tx; const translatedY = y - matrix.ty;
  return { x: (matrix.d * translatedX - matrix.c * translatedY) / determinant, y: (-matrix.b * translatedX + matrix.a * translatedY) / determinant };
}
function matrixShape(matrix: Matrix2D) {
  const scaleX = Math.hypot(matrix.a, matrix.b);
  const determinant = matrix.a * matrix.d - matrix.b * matrix.c;
  return { rotation: Math.atan2(matrix.b, matrix.a), scaleX, scaleY: scaleX === 0 ? 1 : determinant / scaleX };
}

export function computeBoneWorlds(rig: PreviewRig, clip: PreviewClip, tick: number, animated: boolean) {
  const worlds = new Map<string, Matrix2D>(); const visiting = new Set<string>();
  const resolve = (bone: PreviewBone): Matrix2D => {
    const cached = worlds.get(bone.boneId); if (cached) return cached;
    if (visiting.has(bone.boneId)) return localMatrix(0, 0, 0, 1, 1);
    visiting.add(bone.boneId);
    const translate = animated ? samplePreviewTrack(clip.tracks.find(track => track.targetId === bone.boneId && track.channel === 'bone-translate'), tick, [0, 0]) : [0, 0];
    const rotate = animated ? samplePreviewTrack(clip.tracks.find(track => track.targetId === bone.boneId && track.channel === 'bone-rotate'), tick, [0]) : [0];
    const scale = animated ? samplePreviewTrack(clip.tracks.find(track => track.targetId === bone.boneId && track.channel === 'bone-scale'), tick, [1_000_000, 1_000_000]) : [1_000_000, 1_000_000];
    const local = localMatrix(
      bone.rest.xMilliPx / 1000 + (translate[0] ?? 0) / 1000,
      bone.rest.yMilliPx / 1000 + (translate[1] ?? 0) / 1000,
      bone.rest.rotationMilliDeg / 1000 + (rotate[0] ?? 0) / 1000,
      bone.rest.scaleXPpm / 1_000_000 * (scale[0] ?? 1_000_000) / 1_000_000,
      bone.rest.scaleYPpm / 1_000_000 * (scale[1] ?? 1_000_000) / 1_000_000,
    );
    const parent = bone.parentId ? rig.boneTree.bones.find(value => value.boneId === bone.parentId) : undefined;
    const world = parent ? multiply(resolve(parent), local) : local;
    visiting.delete(bone.boneId); worlds.set(bone.boneId, world); return world;
  };
  rig.boneTree.bones.forEach(resolve); return worlds;
}

function colorChannel(value: number | undefined) { return Math.max(0, Math.min(255, Math.round((value ?? 1000) * 255 / 1000))); }

export function PixiRigPreview({ rig, clip, playhead, attachmentPreview }: { rig: PreviewRig; clip: PreviewClip; playhead: number; attachmentPreview: AttachmentPreviewProjection }) {
  const host = useRef<HTMLDivElement>(null);
  const state = useRef({ rig, clip, playhead, attachmentPreview }); state.current = { rig, clip, playhead, attachmentPreview };
  const [runtimeState, setRuntimeState] = useState('正在加载 CAS 附件…');
  const projectionKey = useMemo(() => `${attachmentPreview.layerSetRevision}:${attachmentPreview.rigRevision}:${attachmentPreview.attachments.map(value => value.attachmentSha256).join(':')}`, [attachmentPreview]);
  useEffect(() => {
    let disposed = false; let app: Application | undefined; const loadedUrls: string[] = [];
    void (async () => {
      if (!host.current) return;
      app = new Application();
      await app.init({ resizeTo: host.current, background: 0x111018, antialias: true, resolution: Math.min(window.devicePixelRatio || 1, 2), autoDensity: true });
      if (disposed) { app.destroy(true); for (const url of loadedUrls) void Assets.unload(url); return; }
      host.current.appendChild(app.canvas);
      const scene = new Container(); const attachmentLayer = new Container(); const bonesGraphic = new Graphics();
      attachmentLayer.sortableChildren = true; bonesGraphic.zIndex = 1_000_000; scene.sortableChildren = true;
      scene.addChild(attachmentLayer, bonesGraphic); app.stage.addChild(scene);
      const supported = attachmentPreview.attachments.filter(value => value.supported);
      const loaded = await Promise.all(supported.map(async attachment => {
        const texture = await Assets.load<Texture>(attachment.safePngDataUrl); loadedUrls.push(attachment.safePngDataUrl);
        return { attachment, texture };
      }));
      if (disposed) { app.destroy(true); for (const url of loadedUrls) void Assets.unload(url); return; }
      const bindings = new Map<string, SpriteBinding>();
      for (const { attachment, texture } of loaded) {
        const sprite = new Sprite(texture); sprite.eventMode = 'none';
        sprite.width = rig.canvas.widthPx; sprite.height = rig.canvas.heightPx;
        sprite.anchor.set(attachment.pivot.xMilliPx / 1000 / rig.canvas.widthPx, attachment.pivot.yMilliPx / 1000 / rig.canvas.heightPx);
        const binding = { attachment, sprite, baseScaleX: sprite.scale.x, baseScaleY: sprite.scale.y };
        bindings.set(attachment.slotId, binding); attachmentLayer.addChild(sprite);
      }
      setRuntimeState(`${bindings.size}/${attachmentPreview.attachments.length} 个刚性单骨附件`);
      app.ticker.add(() => {
        if (!app) return;
        const current = state.current;
        const fit = Math.min(app.screen.width / current.rig.canvas.widthPx, app.screen.height / current.rig.canvas.heightPx) * .92;
        scene.scale.set(fit); scene.position.set((app.screen.width - current.rig.canvas.widthPx * fit) / 2, (app.screen.height - current.rig.canvas.heightPx * fit) / 2);
        const restWorlds = computeBoneWorlds(current.rig, current.clip, current.playhead, false);
        const animatedWorlds = computeBoneWorlds(current.rig, current.clip, current.playhead, true);
        for (const binding of bindings.values()) {
          const { attachment, sprite } = binding; const rest = restWorlds.get(attachment.boneId); const animated = animatedWorlds.get(attachment.boneId);
          if (!rest || !animated) { sprite.visible = false; continue; }
          const pivotX = attachment.pivot.xMilliPx / 1000; const pivotY = attachment.pivot.yMilliPx / 1000;
          const localPivot = inverseApply(rest, pivotX, pivotY); const animatedPivot = apply(animated, localPivot.x, localPivot.y);
          const restShape = matrixShape(rest); const animatedShape = matrixShape(animated);
          sprite.position.set(animatedPivot.x, animatedPivot.y); sprite.rotation = animatedShape.rotation - restShape.rotation;
          sprite.scale.set(binding.baseScaleX * animatedShape.scaleX / restShape.scaleX, binding.baseScaleY * animatedShape.scaleY / restShape.scaleY);
          const color = samplePreviewTrack(current.clip.tracks.find(track => track.targetId === attachment.slotId && track.channel === 'slot-color'), current.playhead, [1000, 1000, 1000, 1000]);
          sprite.tint = (colorChannel(color[0]) << 16) | (colorChannel(color[1]) << 8) | colorChannel(color[2]);
          sprite.alpha = Math.max(0, Math.min(1, (color[3] ?? 1000) / 1000)); sprite.visible = attachment.visible && sprite.alpha > 0;
          const order = samplePreviewTrack(current.clip.tracks.find(track => track.targetId === attachment.slotId && track.channel === 'draw-order'), current.playhead, [0]);
          sprite.zIndex = attachment.drawKey + Math.round(order[0] ?? 0);
        }
        attachmentLayer.sortChildren(); bonesGraphic.clear();
        for (const bone of current.rig.boneTree.bones) {
          const point = animatedWorlds.get(bone.boneId); if (!point) continue;
          if (bone.parentId) { const parent = animatedWorlds.get(bone.parentId); if (parent) bonesGraphic.moveTo(parent.tx, parent.ty).lineTo(point.tx, point.ty).stroke({ width: 2 / fit, color: 0xa992ff, alpha: .8 }); }
          bonesGraphic.circle(point.tx, point.ty, 4 / fit).fill({ color: 0x55d7ca, alpha: .9 });
        }
      });
    })().catch(() => { if (!disposed) setRuntimeState('附件纹理加载失败 · 未伪造回退'); });
    return () => { disposed = true; app?.destroy(true, { children: true }); for (const url of loadedUrls) void Assets.unload(url); };
  }, [projectionKey]);
  return <div className="pixi-host" ref={host}><span>CAS LayerSet · {runtimeState}</span></div>;
}
