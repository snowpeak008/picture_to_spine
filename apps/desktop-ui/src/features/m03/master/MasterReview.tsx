import { useState } from 'react';
import { invokeNative } from '../../../native/ipc';
import { useProjectStore, type ProjectProjection } from '../../../state/projectStore';
import type { StyleSpecDraft } from '../M03Gateway';
import { StyleSpecForm } from './StyleSpecForm';
import { MasterApprovalPanel } from './MasterApprovalPanel';
import './master-review.css';

const initial: StyleSpecDraft = {
  viewpoint: 'side-view',
  renderingStyle: 'anime-clean',
  outline: 'dark-clean',
  paletteNotes: '',
  identityNotes: '',
  primaryWeapon: null,
};

interface MasterResult { project: ProjectProjection }
interface MasterPreviewResult extends MasterResult {
  master: NonNullable<ProjectProjection['activeMaster']>;
  approvalPayloadSha256: string;
  safePreviewDataUrl: string;
  reviewToken: string;
  authority: string;
}

function draftFromProject(project: ProjectProjection | null): StyleSpecDraft {
  const style = project?.activeMaster?.styleSpec;
  if (!style) return initial;
  return {
    viewpoint: style.viewpoint,
    renderingStyle: style.renderingStyle,
    outline: style.outline,
    paletteNotes: style.paletteNotes,
    identityNotes: style.identityNotes,
    primaryWeapon: style.primaryWeapon,
  };
}

export function MasterReview() {
  const { project, setProject } = useProjectStore();
  const [value, setValue] = useState<StyleSpecDraft>(() => draftFromProject(project));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [review, setReview] = useState<MasterPreviewResult | null>(null);
  const weapon = value.primaryWeapon;
  const specReady = Boolean(project
    && value.identityNotes.trim()
    && weapon?.weaponType.trim()
    && weapon.socketSemantic.trim()
    && weapon.silhouetteConstraints.trim());
  const candidateReady = project?.masterState === 'PENDING';
  const approved = project?.gates.master === 'APPROVED';

  async function run(method: 'master.create' | 'master.approve' | 'master.reject', payload: Record<string, unknown>) {
    if (!project) return;
    setBusy(true);
    setError('');
    try {
      const result = await invokeNative<MasterResult>(method, payload, project.revision);
      setProject(result.project);
      setReview(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '母版操作失败');
    } finally {
      setBusy(false);
    }
  }

  async function preview() {
    if (!project) return;
    setBusy(true);
    setError('');
    try {
      const result = await invokeNative<MasterPreviewResult>('master.preview', {}, project.revision);
      if (
        result.project.projectId !== project.projectId
        || result.project.revision !== project.revision
        || result.master.masterId !== project.activeMaster?.masterId
        || !result.safePreviewDataUrl.startsWith('data:image/png;base64,')
        || !result.reviewToken
      ) throw new Error('原生母版预览与当前完整候选不匹配');
      setReview(result);
    } catch (reason) {
      setReview(null);
      setError(reason instanceof Error ? reason.message : '母版预览失败');
    } finally {
      setBusy(false);
    }
  }

  return <section className="master-review"><div>{error && <div className="error-banner">{error}</div>}{review&&<figure className="master-authority-preview"><img src={review.safePreviewDataUrl} alt="当前 CAS 母版安全预览"/><figcaption><b>已预览完整母版候选</b><span>{review.authority}</span><code>{review.approvalPayloadSha256}</code></figcaption></figure>}<StyleSpecForm value={value} onChange={setValue} disabled={approved || candidateReady} /></div><MasterApprovalPanel specReady={specReady} candidateReady={candidateReady} reviewed={Boolean(review)} approved={approved} busy={busy} onCreate={() => void run('master.create', { style: { ...value, revision: 0 } })} onPreview={() => void preview()} onApprove={() => void run('master.approve', { reviewToken: review?.reviewToken })} onReject={(reason) => void run('master.reject', { reason })} /></section>;
}
