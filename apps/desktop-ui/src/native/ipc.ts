export type NativeMethod='bootstrap.status'|'remoteGpu.status'|'remoteGpu.importProfile'|'remoteGpu.disable'|'image.chooseAndPreflight'|'image.promote'|'project.create'|'project.open'|'project.recent'|'master.create'|'master.preview'|'master.approve'|'master.reject'|'layers.initialize'|'layers.add'|'layers.delete'|'layers.reorder'|'layers.stroke'|'layers.replacement.chooseAndPreflight'|'layers.replacement.promote'|'layers.status'|'layers.approve'|'rig.initialize'|'rig.status'|'rig.setBone'|'rig.setSlot'|'rig.reparentBone'|'rig.setPivot'|'rig.setSocket'|'rig.approve'|'motion.initialize'|'motion.status'|'motion.spec.update'|'motion.keyPose.chooseAndPreflight'|'motion.keyPose.promote'|'motion.keyPose.alignment.set'|'motion.keyPose.preview'|'motion.keyPose.approve'|'animation.initialize'|'animation.status'|'animation.track.put'|'animation.poseMarker.set'|'animation.hitMarker.set'|'animation.pose.approve'|'animation.hit.approve'|'export.preflight'|'export.chooseRootAndCommit'|'export.history'|'spineCli.status'|'spineCli.selectAndAssess'|'spineCli.clear'|'spineCli.job.start'|'spineCli.job.status'|'diagnostics.status'|'diagnostics.export';

interface NativeResponse<T> {
  schemaVersion: '1.0.0'; requestId: string; ok: boolean; result: T | null;
  error: { code: string; message: string; retryable: boolean } | null;
}
interface WebViewBridge {
  postMessage(message: unknown): void;
  addEventListener(type: 'message', handler: (event: { data: unknown }) => void): void;
}

const pending = new Map<string, { resolve: (value: unknown) => void; reject: (error: Error) => void; timer: number }>();
let listening = false;

function bridge(): WebViewBridge | null {
  return (window as unknown as { chrome?: { webview?: WebViewBridge } }).chrome?.webview ?? null;
}

function ensureListener() {
  const value = bridge();
  if (!value || listening) return;
  listening = true;
  value.addEventListener('message', event => {
    const response = (typeof event.data === 'string' ? JSON.parse(event.data) : event.data) as NativeResponse<unknown>;
    if (response?.schemaVersion !== '1.0.0' || typeof response.requestId !== 'string') return;
    const item = pending.get(response.requestId);
    if (!item) return;
    window.clearTimeout(item.timer);
    pending.delete(response.requestId);
    if (response.ok) item.resolve(response.result);
    else item.reject(new Error(`${response.error?.code ?? 'F2S-IPC'}: ${response.error?.message ?? 'Native command failed'}`));
  });
}

export function nativeAvailable() { return bridge() !== null; }

export function invokeNative<T>(
  method: NativeMethod,
  payload: Record<string, unknown> = {},
  expectedRevision: number | null = null,
  timeoutMs = 30_000,
): Promise<T> {
  const value = bridge();
  if (!value) return Promise.reject(new Error('F2S-IPC-UNAVAILABLE: 当前不是 Windows 桌面宿主'));
  ensureListener();
  const requestId = (globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`).replaceAll('.', '-');
  return new Promise<T>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      pending.delete(requestId);
      reject(new Error('F2S-IPC-TIMEOUT: Native command timed out'));
    }, timeoutMs);
    pending.set(requestId, { resolve: resolve as (value: unknown) => void, reject, timer });
    value.postMessage({ schemaVersion: '1.0.0', requestId, method, expectedRevision, payload });
  });
}
