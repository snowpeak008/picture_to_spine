import { useState } from 'react';

interface Props {
  specReady: boolean;
  candidateReady: boolean;
  reviewed: boolean;
  approved: boolean;
  busy: boolean;
  onCreate: () => void;
  onPreview: () => void;
  onApprove: () => void;
  onReject: (reason: string) => void;
}

export function MasterApprovalPanel({ specReady, candidateReady, reviewed, approved, busy, onCreate, onPreview, onApprove, onReject }: Props) {
  const [reason, setReason] = useState('');
  return <aside className="approval-panel"><header><b>母版审批</b><em>{approved ? '已批准' : reviewed ? '完整候选已预览' : candidateReady ? '必须先预览' : specReady ? '规格可提交' : '缺少规格'}</em></header><p>审批绑定图片、完整 StyleSpec、身份、唯一主武器、candidate revision 与一次性预览令牌。后续修改会使下游批准失效。</p>{!candidateReady && !approved && <button className="primary" disabled={!specReady || busy} onClick={onCreate}>{busy ? '正在提交…' : '创建母版候选'}</button>}{candidateReady && !approved && <><button className="secondary" disabled={busy} onClick={onPreview}>{reviewed ? '重新读取并预览完整母版' : '读取 CAS 并预览完整母版'}</button><button className="primary" disabled={busy || !reviewed} onClick={onApprove}>{busy ? '等待确认…' : '原生确认并批准已预览母版'}</button>{!reviewed&&<small>审批已锁定：必须先查看当前图片与完整 StyleSpec。</small>}<label className="reject-reason">退回原因<textarea value={reason} maxLength={1000} onChange={(event) => setReason(event.target.value)} /></label><button className="secondary" disabled={busy || !reason.trim()} onClick={() => onReject(reason)}>原生确认并退回候选</button></>}</aside>;
}
