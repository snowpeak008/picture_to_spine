import { useCallback, useEffect, useState } from 'react';
import { invokeNative } from '../../native/ipc';
import type { SpineCliStatus } from './spineCliTypes';
import './spine-cli-settings.css';

type SelectionResult = { cancelled: boolean; status: SpineCliStatus };

export function SpineCliSettings() {
  const [status, setStatus] = useState<SpineCliStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const refresh = useCallback(async () => {
    setError('');
    try { setStatus(await invokeNative<SpineCliStatus>('spineCli.status')); }
    catch (cause) { setError(cause instanceof Error ? cause.message : '无法读取 Spine CLI 状态'); }
  }, []);
  useEffect(() => { void refresh(); }, [refresh]);

  async function selectCli() {
    setBusy(true); setError('');
    try {
      const result = await invokeNative<SelectionResult>('spineCli.selectAndAssess', {}, null, 10 * 60_000);
      setStatus(result.status);
    } catch (cause) { setError(cause instanceof Error ? cause.message : 'Spine.com 配置失败'); }
    finally { setBusy(false); }
  }
  async function clearCli() {
    if (!window.confirm('清除本机 Spine CLI 路径和许可确认记录？不会删除 Spine。')) return;
    setBusy(true); setError('');
    try { setStatus(await invokeNative<SpineCliStatus>('spineCli.clear')); }
    catch (cause) { setError(cause instanceof Error ? cause.message : '无法清除 Spine CLI 配置'); }
    finally { setBusy(false); }
  }
  const assessment = status?.assessment;
  return <section className="spine-cli-settings" aria-labelledby="spine-cli-settings-title">
    <header><div><span className="eyebrow">USER-OWNED PROFESSIONAL CLI · EXTERNAL</span><h1 id="spine-cli-settings-title">Spine Professional CLI 4.2.43</h1><p>仅保存用户原生选择的本机 <code>Spine.com</code>。应用不捆绑、不下载、不读取激活信息，也不会把绝对路径发送给 WebView。</p></div><div className="spine-cli-settings-actions"><button className="secondary" disabled={busy} onClick={() => void refresh()}>刷新</button><button className="primary" disabled={busy} onClick={() => void selectCli()}>{busy ? '等待原生操作…' : status?.configured ? '重新选择 Spine.com' : '选择 Spine.com'}</button><button className="danger-secondary" disabled={busy || !status?.configured} onClick={() => void clearCli()}>清除配置</button></div></header>
    {error && <div className="error-banner" role="alert">{error}</div>}
    <div className="spine-cli-status-grid"><article><span>外部证据状态</span><strong>{assessment?.state ?? 'NOT_RUN'}</strong><small>{assessment?.reasonCode ?? 'READING_LOCAL_CONFIG'}</small></article><article><span>固定目标 patch</span><strong>{assessment?.expectedPatch ?? '4.2.43'}</strong><small>运行前后均需精确观测</small></article><article><span>Professional 许可确认</span><strong>{assessment?.professionalLicenseConfirmed ? 'CONFIRMED' : 'NOT_CONFIRMED'}</strong><small>由 Windows 原生确认；不检查激活数据</small></article><article><span>本轮真实 CLI 探测</span><strong>{assessment?.realCliTested ? assessment.observedPatch ?? 'FAILED' : 'NOT_RUN'}</strong><small>选择可执行文件不等于 Editor 验证通过</small></article></div>
    <dl className="spine-cli-identities"><div><dt>Path token</dt><dd><code>{assessment?.pathToken ?? 'NOT_CONFIGURED'}</code></dd></div><div><dt>Executable SHA-256</dt><dd><code>{assessment?.executableSha256 ?? 'NOT_CONFIGURED'}</code></dd></div></dl>
    <aside className="spine-cli-boundary"><strong>边界说明</strong><p>配置完成后状态仍是 EXTERNAL / NOT_RUN。只有某次导出 job 在同一轮人工确认后完成前后版本探测、生成预期专有扩展，并通过逐文件 provenance hash 授权，界面才会将该 job 标记为成功。</p></aside>
  </section>;
}
