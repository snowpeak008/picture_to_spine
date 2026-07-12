import { useEffect, useState } from 'react';
import { invokeNative, nativeAvailable } from '../../native/ipc';
import './diagnostics.css';

interface NativeDiagnostics {
  schemaVersion: string;
  ipc: string;
  imageDecode: string;
  worker: string;
  privateRemoteGpu: string;
  spineEditor: string;
  projectIntegrity?: string;
  networkCallCount: number;
  evidence: {
    ipc: string;
    imageDecode: string;
    projectIntegrity: string;
    networkCallCount: string;
    spineEditor: string;
    worker: string;
  };
}

interface DiagnosticsExportResult {
  cancelled: boolean;
  fileName?: string;
  sha256?: string;
  bytes?: number;
  status?: string;
}

export function DiagnosticsPage() {
  const [data, setData] = useState<NativeDiagnostics | null>(null);
  const [error, setError] = useState('');
  const [exporting, setExporting] = useState(false);
  const [exported, setExported] = useState<DiagnosticsExportResult | null>(null);

  useEffect(() => {
    if (!nativeAvailable()) {
      setError('Native IPC 不可用');
      return;
    }
    void invokeNative<NativeDiagnostics>('diagnostics.status')
      .then(setData)
      .catch((reason) => setError(reason instanceof Error ? reason.message : '诊断失败'));
  }, []);

  async function exportReport() {
    if (!data) return;
    setExporting(true);
    setError('');
    try {
      const result = await invokeNative<DiagnosticsExportResult>('diagnostics.export');
      if (!result.cancelled) setExported(result);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '诊断报告导出失败');
    } finally {
      setExporting(false);
    }
  }

  const rows = [
    ['Native IPC', data?.ipc ?? '未观测', data?.evidence.ipc ?? 'UNVERIFIED'],
    ['图片解码', data?.imageDecode ?? '未观测', data?.evidence.imageDecode ?? 'UNVERIFIED'],
    ['项目完整性', data?.projectIntegrity ?? '未观测', data?.evidence.projectIntegrity ?? 'UNVERIFIED'],
    ['Spine Editor', data?.spineEditor ?? '未观测', data?.evidence.spineEditor ?? 'EXTERNAL'],
    ['AppContainer Worker', data?.worker ?? 'UNVERIFIED_EXCLUDED', data?.evidence.worker ?? 'UNVERIFIED'],
    ['私有远程 GPU', data?.privateRemoteGpu ?? 'DISABLED', 'EXTERNAL_STATUS_PROJECTION'],
    ['宿主网络调用计数', data ? String(data.networkCallCount) : '未观测', data?.evidence.networkCallCount ?? 'UNVERIFIED'],
  ];

  return <section className="diagnostics">
    <header>
      <div>
        <span className="eyebrow">离线运行时诊断</span>
        <h1>能力与外部条件</h1>
        <p>下表就是导出前预览清单。报告只含版本、能力状态、计数和脱敏项目摘要，不含图片、PromptPack 正文、凭据、用户名或绝对路径。</p>
      </div>
      <button className="secondary" disabled={!data || exporting} onClick={() => void exportReport()}>
        {exporting ? '正在生成…' : '选择位置并导出脱敏 JSON'}
      </button>
    </header>
    {error && <div className="error-banner" role="alert">{error}</div>}
    {exported && <div className="diagnostics-exported" aria-live="polite"><b>{exported.status}</b><span>{exported.fileName} · {exported.bytes} bytes</span><code>{exported.sha256}</code></div>}
    <div className="diag-table">{rows.map(([name, version, state]) => <div key={name}><b>{name}</b><span>{version}</span><em className={state.startsWith('OBSERVED') ? 'observed' : ''}>{state}</em></div>)}</div>
    <aside><strong>安全提示</strong><p>诊断不会安装工具、修改 ExecutionPolicy、读取 Spine 激活信息或连接公网。保存位置由 Windows 原生对话框明确选择。</p></aside>
  </section>;
}
