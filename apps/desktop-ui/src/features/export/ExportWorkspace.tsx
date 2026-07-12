import { useEffect, useMemo, useState } from 'react';
import { invokeNative } from '../../native/ipc';
import { useProjectStore, type ProjectProjection } from '../../state/projectStore';
import { cliJobTerminal, type SpineCliJob, type SpineCliStatus } from '../settings/spineCliTypes';
import './export.css';

interface ExportPreflight { passed: boolean; checks: string[]; errors: string[]; externalEditorStatus: string; publishStatus: string }
interface ExportOutput { path: string; owner: string; state: 'READY_FOR_COMMIT' | 'BLOCKED' | 'EXTERNAL' }
interface ExportRecord { exportId: string; snapshotSha256: string; sourceProjectRevision: number; status: string; checksums: Record<string, string>; createdAtUtc: string; externalStatus: string }
interface ExportProjection { project: ProjectProjection; preflight: ExportPreflight; snapshotSha256: string | null; outputs: ExportOutput[]; history: ExportRecord[]; authority: string }
interface ExportCommitResponse { cancelled: boolean; project: ProjectProjection; preflight: ExportPreflight; exportId?: string; snapshotSha256?: string; directoryToken?: string; status?: string; externalEditorStatus?: string; checksums?: Record<string, string>; history: ExportRecord[] }
type SelectionResult = { cancelled: boolean; status: SpineCliStatus };
type CliOperationKind = 'IMPORT_PROJECT' | 'PACK_ATLAS' | 'EXPORT_BINARY';

const operationLabels: Record<CliOperationKind, { title: string; detail: string }> = {
  IMPORT_PROJECT: { title: '生成 .spine', detail: '从本轮开放包的 character.spine.json 导入；输出到新的外部目录。' },
  PACK_ATLAS: { title: '生成 .atlas', detail: '打包本轮开放包 images；需原生选择 pack settings JSON。' },
  EXPORT_BINARY: { title: '生成 .skel', detail: '原生选择一个 .spine 项目；输出到新的外部目录。' },
};

export function ExportWorkspace() {
  const { project, setProject } = useProjectStore();
  const [data, setData] = useState<ExportProjection | null>(null);
  const [lastCommit, setLastCommit] = useState<ExportCommitResponse | null>(null);
  const [cliStatus, setCliStatus] = useState<SpineCliStatus | null>(null);
  const [cliJob, setCliJob] = useState<SpineCliJob | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  async function refresh(revision = project?.revision) {
    if (!project || revision === undefined) return;
    setBusy(true); setError('');
    try {
      const [projection, status] = await Promise.all([
        invokeNative<ExportProjection>('export.preflight', {}, revision),
        invokeNative<SpineCliStatus>('spineCli.status'),
      ]);
      setProject(projection.project); setData(projection); setCliStatus(status);
    } catch (cause) { setError(cause instanceof Error ? cause.message : '导出预检失败'); }
    finally { setBusy(false); }
  }

  async function commit() {
    if (!project || !data?.preflight.passed) return;
    setBusy(true); setError('');
    try {
      const result = await invokeNative<ExportCommitResponse>('export.chooseRootAndCommit', {}, project.revision, 10 * 60_000);
      setProject(result.project);
      if (result.cancelled) return;
      setLastCommit(result);
      const [next, status] = await Promise.all([
        invokeNative<ExportProjection>('export.preflight', {}, result.project.revision),
        invokeNative<SpineCliStatus>('spineCli.status'),
      ]);
      setData(next); setCliStatus(status); setProject(next.project);
    } catch (cause) { setError(cause instanceof Error ? cause.message : '开放格式导出失败'); }
    finally { setBusy(false); }
  }

  async function selectCli() {
    setBusy(true); setError('');
    try {
      const result = await invokeNative<SelectionResult>('spineCli.selectAndAssess', {}, null, 10 * 60_000);
      setCliStatus(result.status);
    } catch (cause) { setError(cause instanceof Error ? cause.message : 'Spine.com 配置失败'); }
    finally { setBusy(false); }
  }

  const currentOpenExport = useMemo(() => {
    if (!project || !cliStatus) return null;
    const candidates = cliStatus.openExports.filter(item => item.projectId === project.projectId && item.projectRevision === project.revision);
    return candidates.find(item => item.exportId === lastCommit?.exportId) ?? candidates[0] ?? null;
  }, [cliStatus, lastCommit?.exportId, project]);

  async function startCliOperation(operationKind: CliOperationKind) {
    if (!project || !currentOpenExport || !cliStatus?.configured) return;
    setError('');
    try {
      const job = await invokeNative<SpineCliJob>('spineCli.job.start', { exportId: currentOpenExport.exportId, operationKind }, project.revision);
      setCliJob(job);
    } catch (cause) { setError(cause instanceof Error ? cause.message : '无法启动 Spine CLI job'); }
  }

  useEffect(() => { if (project) void refresh(project.revision); }, [project?.projectId]);
  useEffect(() => {
    if (!cliJob || cliJobTerminal(cliJob.state)) return;
    let inFlight = false;
    const timer = window.setInterval(() => {
      if (inFlight) return;
      inFlight = true;
      void invokeNative<SpineCliJob>('spineCli.job.status', { jobId: cliJob.jobId })
        .then(setCliJob)
        .catch(cause => setError(cause instanceof Error ? cause.message : '无法读取 CLI job 状态'))
        .finally(() => { inFlight = false; });
    }, 700);
    return () => window.clearInterval(timer);
  }, [cliJob]);

  if (!project) return <section className="export-workspace"><div className="blocked-panel"><b>导出工作台已锁定</b><p>请先创建或打开本地项目。</p></div></section>;
  if (!data) return <section className="export-workspace"><header><div><span className="eyebrow">Spine 4.2.43</span><h1>导出与外部 CLI</h1><p>正在从项目事实源重算审批闭包与输出合同。</p></div><button className="secondary" disabled={busy} onClick={() => void refresh()}>{busy ? '正在预检…' : '重新预检'}</button></header>{error && <div role="alert" className="error-banner">{error}</div>}</section>;

  const passed = data.preflight.passed;
  const cliActive = cliJob !== null && !cliJobTerminal(cliJob.state);
  return <section className="export-workspace">
    <header><div><span className="eyebrow">{data.authority}</span><h1>导出与外部 CLI</h1><p>先提交不可变开放包；专有扩展只由用户本机合法的 Spine Professional 4.2.43 在独立外部目录生成。</p></div><div className="export-actions"><button className="secondary" disabled={busy} onClick={() => void refresh()}>{busy ? '正在验证…' : '重新预检'}</button><button className="primary" disabled={busy || !passed} onClick={() => void commit()}>{busy ? '等待原生操作…' : '选择目录并提交开放包'}</button></div></header>
    {error && <div role="alert" className="error-banner">{error}<button onClick={() => setError('')}>关闭</button></div>}
    {lastCommit && !lastCommit.cancelled && <section className="export-success" aria-live="polite"><b>不可变开放包已完成</b><p>目录 token：<code>{lastCommit.directoryToken}</code></p><code>{lastCommit.exportId} · {lastCommit.snapshotSha256}</code><span>{lastCommit.status} · Editor {lastCommit.externalEditorStatus}</span></section>}
    <div className="export-grid"><main>
      <section className="export-card"><header><b>PublishSnapshot Preflight</b><em className={passed ? 'ok' : ''}>{passed ? 'PASS · 可提交' : 'BLOCKED'}</em></header><div className="preflight-list">{data.preflight.checks.map(check => <p key={check}><i className="ok">✓</i><span>{labelCheck(check)}</span><code>{check}</code></p>)}{data.preflight.errors.map(issue => <p key={issue}><i>●</i><span>{labelError(issue)}</span><code>{issue}</code></p>)}</div>{data.snapshotSha256 && <footer><span>候选快照 SHA-256</span><code>{data.snapshotSha256}</code></footer>}</section>
      <section className="export-card"><header><b>开放包输出清单</b><span>状态来自 Native 预检，不代表文件已生成</span></header><div className="output-table">{data.outputs.map(output => <div key={output.path}><code>{output.path}</code><span>{output.owner}</span><em className={output.state === 'READY_FOR_COMMIT' ? 'ok' : 'external'}>{output.state}</em></div>)}</div></section>
      <section className="export-card cli-runner-card"><header><b>用户本地 Professional CLI</b><span>{cliStatus?.configured ? '已选择 · 尚未运行' : '未配置'}</span></header>
        <div className="cli-runner-identity"><div><span>证据</span><strong>{cliStatus?.assessment.state ?? 'NOT_RUN'} / EXTERNAL</strong></div><div><span>Path token</span><code>{cliStatus?.assessment.pathToken ?? 'NOT_CONFIGURED'}</code></div><div><span>Executable SHA-256</span><code>{cliStatus?.assessment.executableSha256 ?? 'NOT_CONFIGURED'}</code></div><button className="secondary" disabled={busy || cliActive} onClick={() => void selectCli()}>{cliStatus?.configured ? '重新选择 Spine.com' : '选择 Spine.com'}</button></div>
        {!currentOpenExport && <div className="cli-prerequisite">请先在本轮会话提交一个开放包。CLI 输出不会写入开放包内部。</div>}
        <div className="cli-operation-grid">{(Object.keys(operationLabels) as CliOperationKind[]).map(kind => <article key={kind}><b>{operationLabels[kind].title}</b><p>{operationLabels[kind].detail}</p><button className="primary" disabled={!cliStatus?.configured || !currentOpenExport || cliActive} onClick={() => void startCliOperation(kind)}>{cliActive ? '已有 job 运行中' : '启动原生 job'}</button></article>)}</div>
        {cliJob && <div className={`cli-job-result ${cliJob.state.toLowerCase()}`} aria-live="polite"><header><b>{cliJob.operationKind}</b><em>{cliJob.state}</em></header><dl><div><dt>Job</dt><dd><code>{cliJob.jobId}</code></dd></div><div><dt>Operation</dt><dd><code>{cliJob.operationId}</code></dd></div><div><dt>Failure</dt><dd>{cliJob.failureCode ?? 'NONE'}</dd></div><div><dt>Output path token</dt><dd><code>{cliJob.outputPathToken ?? 'NOT_AVAILABLE'}</code></dd></div><div><dt>Provenance SHA-256</dt><dd><code>{cliJob.provenanceSha256 ?? 'NOT_AVAILABLE'}</code></dd></div></dl>{cliJob.outputs.map(output => <p key={output.relativePath}><code>{output.relativePath}</code><code>{output.sha256}</code><strong>{output.authorized ? 'AUTHORIZED' : 'REJECTED'}</strong></p>)}</div>}
      </section>
      {data.history.length > 0 && <section className="export-card export-history"><header><b>项目导出历史</b><span>{data.history.length} 个不可变记录</span></header>{[...data.history].reverse().map(record => <article key={record.exportId}><div><b>{record.exportId}</b><span>{record.createdAtUtc}</span></div><code>{record.snapshotSha256}</code><p>源 revision {record.sourceProjectRevision} · {Object.keys(record.checksums).length + 1} 个包文件 · {record.status} · Editor {record.externalStatus}</p></article>)}</section>}
    </main><aside><section className="status-contract"><b>状态合同</b><dl><div><dt>内置 writer</dt><dd className={passed ? 'ok' : ''}>{passed ? 'CONTRACT_AVAILABLE' : 'BLOCKED'}</dd></div><div><dt>本轮开放包</dt><dd>{lastCommit?.status ?? 'NOT_EXPORTED'} · EXPORTED_UNVERIFIED</dd></div><div><dt>Professional CLI</dt><dd>{cliJob?.state ?? 'EXTERNAL / NOT_RUN'}</dd></div><div><dt>Release proof</dt><dd>不可构造</dd></div></dl></section><section className="professional-boundary"><b>Professional CLI 边界</b><p>应用不捆绑、不下载、不读取激活信息。每次 job 先哈希输入并生成 consent binding，再由原生人工确认。只有精确观测 4.2.43 且逐项 provenance 授权才显示成功。</p></section><section className="warning-contract"><b>不会导出的内容</b><p>伤害数值、碰撞判定、连招状态机、敌人反应、引擎控制器和游戏运行时代码均不属于 Spine 动画包。</p></section></aside></div>
  </section>;
}

function labelCheck(value: string) { return ({ SPINE_PATCH_EXACT: 'Spine patch 精确固定为 4.2.43', APPROVAL_CLOSURE_VALID: '十个 Pose 与三个 Hit 审批闭包有效', BUILTIN_OUTPUT_SUBSET_OPEN: '内置输出只含开放格式', PATHS_CONFINED: '附件路径限定在安全 images 子目录' } as Record<string, string>)[value] ?? value; }
function labelError(value: string) { if (value.includes('POSE_APPROVAL_MISSING')) return '仍有动作关键姿势未审批'; if (value.includes('HIT_APPROVAL_MISSING')) return '仍有攻击命中帧未审批'; if (value.includes('RIG_APPROVAL')) return '当前 Rig 审批无效或缺失'; if (value.includes('LAYER_APPROVAL')) return '当前分层审批无效或缺失'; if (value.includes('MASTER_APPROVAL')) return '当前母版审批无效或缺失'; if (value.includes('MULTI_BONE')) return '当前 writer 暂不支持多骨骼局部坐标权重'; return '项目快照未满足导出不变量'; }
