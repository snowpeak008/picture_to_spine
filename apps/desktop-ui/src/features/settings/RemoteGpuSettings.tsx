import { useCallback, useEffect, useState } from 'react';
import { invokeNative } from '../../native/ipc';
import './remote-gpu-settings.css';

type RemoteGpuMethod =
  | 'LAYER_SEGMENTATION_CANDIDATE'
  | 'RIG_PROPOSAL_CANDIDATE'
  | 'MOTION_CURVE_CANDIDATE';

type RemoteGpuProfile = {
  schemaVersion: '1.0.0';
  enabled: boolean;
  profileId: string;
  ownership: 'USER_CONTROLLED_PRIVATE';
  origin: string;
  allowedPorts: number[];
  certificateSpkiSha256: string;
  organizationIdentitySha256: string;
  credentialManagerTarget: string;
  allowedMethods: RemoteGpuMethod[];
  allowedInputMediaTypes: string[];
  allowedModelManifestSha256: string[];
  maxUploadBytes: number;
  maxResponseBytes: number;
  requestTimeoutSeconds: number;
};

type RemoteGpuStatus = {
  schemaVersion: '1.0.0';
  activeProfile: RemoteGpuProfile | null;
  credentialConfigured: boolean | null;
  capability: {
    state: 'NOT_RUN_EXTERNAL' | 'CONTRACT_MOCK_ONLY';
    transport: 'EXTERNAL_NOT_INTEGRATED';
    reason: string;
    networkAttemptCount: 0;
    networkAttempted: false;
  };
  policy: {
    automaticConnection: false;
    transportCredentialRead: false;
    credentialPresenceCheck: 'NOT_RUN_DEFAULT_NO_CREDENTIAL_READ';
    publicProviderAllowed: false;
    imageGenerationAllowed: false;
    secretAcceptedByIpc: false;
    profileStorage: 'LOCAL_APPDATA_ONLY_NOT_PROJECT';
  };
};

type ImportResult = { cancelled: boolean; status: RemoteGpuStatus };

const methodLabels: Record<RemoteGpuMethod, string> = {
  LAYER_SEGMENTATION_CANDIDATE: '分层候选',
  RIG_PROPOSAL_CANDIDATE: 'Rig 提案候选',
  MOTION_CURVE_CANDIDATE: '动作曲线候选',
};

function formatBytes(value: number) {
  if (value >= 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${value} B`;
}

function HashList({ values }: { values: string[] }) {
  return <div className="remote-hash-list">{values.map(value => <code key={value}>{value}</code>)}</div>;
}

export function RemoteGpuSettings() {
  const [status, setStatus] = useState<RemoteGpuStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const refresh = useCallback(async () => {
    setError('');
    try {
      setStatus(await invokeNative<RemoteGpuStatus>('remoteGpu.status'));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : '私有远程 GPU 状态读取失败');
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  async function importProfile() {
    setBusy(true);
    setError('');
    try {
      const result = await invokeNative<ImportResult>('remoteGpu.importProfile');
      setStatus(result.status);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : '配置导入失败');
    } finally {
      setBusy(false);
    }
  }

  async function disableProfile() {
    if (!status?.activeProfile || !window.confirm(`停用配置 ${status.activeProfile.profileId}？配置与本地审计快照会保留。`)) return;
    setBusy(true);
    setError('');
    try {
      setStatus(await invokeNative<RemoteGpuStatus>('remoteGpu.disable'));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : '配置停用失败');
    } finally {
      setBusy(false);
    }
  }

  const profile = status?.activeProfile ?? null;
  return <section className="remote-settings" aria-labelledby="remote-settings-title">
    <header className="remote-settings-header">
      <div>
        <span className="eyebrow">LOCAL PRIVATE CONFIGURATION</span>
        <h1 id="remote-settings-title">私有远程 GPU</h1>
        <p>这里只管理本机配置。导入与停用均不会连接端点；真实网络传输保持 NOT_RUN / EXTERNAL。</p>
      </div>
      <div className="remote-settings-actions">
        <button className="secondary" disabled={busy} onClick={() => void refresh()}>刷新状态</button>
        <button className="primary" disabled={busy} onClick={() => void importProfile()}>{busy ? '处理中…' : '导入本地 Profile JSON'}</button>
      </div>
    </header>

    {error && <div className="error-banner" role="alert">{error}</div>}

    <div className="remote-capability-grid">
      <article>
        <span>真实 Transport</span>
        <strong>{status?.capability.state ?? '读取中…'}</strong>
        <small>{status?.capability.transport ?? 'EXTERNAL_NOT_INTEGRATED'}</small>
      </article>
      <article>
        <span>网络尝试数</span>
        <strong>{status?.capability.networkAttemptCount ?? 0}</strong>
        <small>默认不连接 · 不自动重试</small>
      </article>
      <article>
        <span>Windows 凭据条目</span>
        <strong>{status?.credentialConfigured === true ? '已配置' : status?.credentialConfigured === false ? '未配置' : '未检查'}</strong>
        <small>默认不读取凭据；存在性检查保持 NOT_RUN</small>
      </article>
      <article>
        <span>存储边界</span>
        <strong>LOCALAPPDATA</strong>
        <small>不写入项目，不接受 IPC secret</small>
      </article>
    </div>

    <aside className="remote-policy-note">
      <strong>能力边界</strong>
      <p>{status?.capability.reason ?? '正在读取 Native 权威状态…'}</p>
      <ul>
        <li>只允许用户控制的私有 HTTPS origin 与固定候选方法。</li>
        <li>不允许公网 AI provider，不允许生图，不接受 fallback。</li>
        <li>Profile 中只保存 Credential Manager target 引用；凭据内容不会进入 WebView。</li>
      </ul>
    </aside>

    {!profile ? <div className="remote-empty">
      <strong>尚未导入活动配置</strong>
      <p>请选择由管理员准备的严格 JSON；Native 会完整反序列化并执行私有端点、证书 pin、方法和字节预算校验。</p>
    </div> : <article className="remote-profile-card">
      <header>
        <div><span>活动 Profile</span><h2>{profile.profileId}</h2></div>
        <em className={profile.enabled ? 'enabled' : 'disabled'}>{profile.enabled ? 'ENABLED（未连接）' : 'DISABLED'}</em>
        <button className="danger-secondary" disabled={busy || !profile.enabled} onClick={() => void disableProfile()}>停用并保留配置</button>
      </header>
      <dl className="remote-profile-fields">
        <div><dt>Schema / Ownership</dt><dd>{profile.schemaVersion} · {profile.ownership}</dd></div>
        <div><dt>Private HTTPS Origin</dt><dd><code>{profile.origin}</code></dd></div>
        <div><dt>允许端口</dt><dd>{profile.allowedPorts.join(', ')}</dd></div>
        <div><dt>Credential Manager Target</dt><dd><code>{profile.credentialManagerTarget}</code></dd></div>
        <div className="wide"><dt>证书 SPKI SHA-256</dt><dd><code>{profile.certificateSpkiSha256}</code></dd></div>
        <div className="wide"><dt>组织身份 SHA-256</dt><dd><code>{profile.organizationIdentitySha256}</code></dd></div>
        <div className="wide"><dt>候选方法</dt><dd className="remote-pills">{profile.allowedMethods.map(method => <span key={method}>{methodLabels[method]}</span>)}</dd></div>
        <div className="wide"><dt>输入媒体白名单</dt><dd className="remote-pills">{profile.allowedInputMediaTypes.map(type => <span key={type}>{type}</span>)}</dd></div>
        <div className="wide"><dt>允许的模型清单 SHA-256</dt><dd><HashList values={profile.allowedModelManifestSha256}/></dd></div>
        <div><dt>最大上传</dt><dd>{formatBytes(profile.maxUploadBytes)}</dd></div>
        <div><dt>最大响应</dt><dd>{formatBytes(profile.maxResponseBytes)}</dd></div>
        <div><dt>请求超时</dt><dd>{profile.requestTimeoutSeconds} 秒</dd></div>
        <div><dt>自动连接</dt><dd>禁止</dd></div>
      </dl>
    </article>}
  </section>;
}
