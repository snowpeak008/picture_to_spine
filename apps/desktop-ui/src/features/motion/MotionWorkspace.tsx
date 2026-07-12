import { useEffect, useState } from 'react';
import { invokeNative } from '../../native/ipc';
import { useProjectStore, type ProjectProjection } from '../../state/projectStore';
import './motion.css';
import './motion-real.css';

interface ActionDefinition {
  key: string;
  category: string;
  loops: boolean;
  requiresHitFrame: boolean;
}

interface MotionPhase {
  key: string;
  startTick: number;
  endTick: number;
  intent: string;
}

interface MotionSpec {
  actionKey: string;
  revision: number;
  durationTicks: number;
  timeBase: { numerator: number; denominator: number };
  loopPolicy: string;
  rootMotion: string;
  silhouetteGoal: string;
  weaponIntent: string | null;
  phases: MotionPhase[];
  contactTicks: number[];
}

interface AssetSpec {
  assetSpecId: string;
  actionKey: string;
  poseKey: string;
  required: boolean;
  purpose: string;
  state: 'missing' | 'requested' | 'imported' | 'reviewed' | 'approved';
}

interface PromptEntry {
  assetSpecId: string;
  actionKey: string;
  poseKey: string;
  positive: string;
  negative: string;
}

interface KeyPoseBinding {
  bindingId: string;
  revision: number;
  assetSpecId: string;
  actionKey: string;
  poseKey: string;
  sourceSha256: string;
  mediaType: string;
  width: number;
  height: number;
  groundYMilliPx: number;
  scalePpm: number;
}

interface MotionContent {
  revision: number;
  specs: MotionSpec[];
  strategies: Array<{
    actionKey: string;
    part: string;
    strategy: string;
    ruleId: string;
    capabilityId: string;
    explanation: string;
  }>;
  assets: AssetSpec[];
  promptPack: {
    packId: string;
    revision: number;
    styleSha256: string;
    motionRevisionHash: string;
    providerProfile: string;
    entries: PromptEntry[];
    networkCallsMade: number;
  };
  keyPoseBindings: KeyPoseBinding[];
}

interface MatrixRow {
  actionKey: string;
  specReady: boolean;
  promptReady: boolean;
  requiredAssets: number;
  approvedAssets: number;
  ready: boolean;
  reason: string;
}

interface MotionResponse {
  project: ProjectProjection;
  registry: ActionDefinition[];
  content: MotionContent;
  matrix: MatrixRow[];
  authority: string;
}

interface ChooseResult {
  cancelled: boolean;
  stagingToken?: string;
  fileName?: string;
  report?: {
    mediaType: string;
    width: number;
    height: number;
    sourceSha256: string;
    completeDecode: boolean;
  };
}

interface KeyPosePreview {
  projectId: string;
  projectRevision: number;
  binding: KeyPoseBinding;
  safePreviewDataUrl: string;
  reviewToken: string;
  authority: string;
}

type MotionCommand =
  | 'motion.initialize'
  | 'motion.status'
  | 'motion.spec.update'
  | 'motion.keyPose.promote'
  | 'motion.keyPose.alignment.set'
  | 'motion.keyPose.approve';

export function MotionWorkspace() {
  const { project, setProject } = useProjectStore();
  const [data, setData] = useState<MotionResponse | null>(null);
  const [selected, setSelected] = useState('attack_01');
  const [tab, setTab] = useState<'spec' | 'bom' | 'prompt' | 'import'>('prompt');
  const [draft, setDraft] = useState<MotionSpec | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [staged, setStaged] = useState<Record<string, { token: string; label: string }>>({});
  const [reviews, setReviews] = useState<Record<string, KeyPosePreview>>({});

  async function run(method: MotionCommand, payload: Record<string, unknown> = {}) {
    if (!project) return;
    setBusy(true);
    setError('');
    try {
      const value = await invokeNative<MotionResponse>(method, payload, project.revision);
      setProject(value.project);
      setData(value);
      const spec =
        value.content.specs.find((item) => item.actionKey === selected) ?? value.content.specs[0];
      setSelected(spec.actionKey);
      setDraft(structuredClone(spec));
      // Any successful command may move the project head. Native review grants never survive it.
      setReviews({});
      if (method === 'motion.keyPose.promote' && typeof payload.stagingToken === 'string') {
        setStaged((current) =>
          Object.fromEntries(
            Object.entries(current).filter(([, entry]) => entry.token !== payload.stagingToken),
          ),
        );
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'MotionContent 命令失败');
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (project?.gates.master === 'APPROVED' && project.motionState === 'PRESENT') {
      void run('motion.status');
    }
  }, [project?.projectId]);

  async function choose(asset: AssetSpec) {
    if (!project) return;
    setBusy(true);
    setError('');
    setReviews({});
    try {
      const value = await invokeNative<ChooseResult>(
        'motion.keyPose.chooseAndPreflight',
        { assetSpecId: asset.assetSpecId },
        project.revision,
      );
      if (!value.cancelled && value.stagingToken) {
        setStaged((current) => ({
          ...current,
          [asset.assetSpecId]: {
            token: value.stagingToken!,
            label: `${value.fileName} · ${value.report?.width}×${value.report?.height}`,
          },
        }));
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '关键姿势图片预检失败');
    } finally {
      setBusy(false);
    }
  }

  async function preview(binding: KeyPoseBinding) {
    if (!project) return;
    setBusy(true);
    setError('');
    try {
      const value = await invokeNative<KeyPosePreview>(
        'motion.keyPose.preview',
        { bindingId: binding.bindingId },
        project.revision,
      );
      const matchesCurrentBinding =
        value.projectId === project.projectId &&
        value.projectRevision === project.revision &&
        value.binding.bindingId === binding.bindingId &&
        value.binding.sourceSha256 === binding.sourceSha256 &&
        value.binding.actionKey === binding.actionKey &&
        value.binding.poseKey === binding.poseKey;
      if (
        !matchesCurrentBinding ||
        !value.safePreviewDataUrl.startsWith('data:image/png;base64,') ||
        !value.reviewToken
      ) {
        throw new Error('原生预览返回了不匹配的关键姿势绑定');
      }
      setReviews((current) => ({ ...current, [binding.bindingId]: value }));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '关键姿势图片预览失败');
    } finally {
      setBusy(false);
    }
  }

  async function approve(binding: KeyPoseBinding) {
    const review = reviews[binding.bindingId];
    if (!review) {
      setError('必须先预览当前绑定，才能提交人工审批');
      return;
    }
    // Native consumes the token before opening its confirmation dialog. Clear the UI copy first too.
    setReviews({});
    await run('motion.keyPose.approve', {
      bindingId: binding.bindingId,
      reviewToken: review.reviewToken,
    });
  }

  if (!project || project.gates.master !== 'APPROVED') {
    return (
      <section className="motion-workspace">
        <div className="blocked-panel">
          <b>MotionContent 已锁定</b>
          <p>需要当前母版及主武器语义的有效审批。</p>
        </div>
      </section>
    );
  }

  if (project.motionState === 'MISSING' && !data) {
    return (
      <section className="motion-workspace">
        <header>
          <div>
            <span className="eyebrow">内容规格 · 纯离线提示词</span>
            <h1>十动作内容与素材计划</h1>
            <p>
              从用户批准的 StyleSpec 和主武器生成结构化 MotionSpec、BOM 与
              provider-neutral 提示词；不会调用图片 API。
            </p>
          </div>
          <button className="primary" disabled={busy} onClick={() => void run('motion.initialize')}>
            {busy ? '正在生成…' : '创建十动作内容规格'}
          </button>
        </header>
        {error && <div className="error-banner" role="alert">{error}</div>}
      </section>
    );
  }

  if (!data || !draft) {
    return (
      <section className="motion-workspace">
        <div className="blocked-panel">
          <b>正在验证 MotionContent</b>
          <p>{error || '重算 StyleSpec/MotionSpec/PromptPack 哈希…'}</p>
        </div>
      </section>
    );
  }

  const definition = data.registry.find((item) => item.key === selected)!;
  const spec = data.content.specs.find((item) => item.actionKey === selected)!;
  const assets = data.content.assets.filter((item) => item.actionKey === selected);
  const prompts = data.content.promptPack.entries.filter((item) => item.actionKey === selected);
  const strategies = data.content.strategies.filter((item) => item.actionKey === selected);
  const matrix = data.matrix.find((item) => item.actionKey === selected)!;

  const selectAction = (key: string) => {
    setSelected(key);
    const value = data.content.specs.find((item) => item.actionKey === key);
    if (value) setDraft(structuredClone(value));
  };

  return (
    <section className="motion-workspace">
      <header>
        <div>
          <span className="eyebrow">{data.authority}</span>
          <h1>十动作内容与素材计划</h1>
          <p>只输出动作描述、BOM 和 AI 开发提示词；关键姿势图片必须从本地导入、真实预览并逐张人工审批。</p>
        </div>
        <div className="motion-pack-id">
          <span>PromptPack</span>
          <code>{data.content.promptPack.packId}</code>
          <small>{data.content.promptPack.networkCallsMade} 次网络调用</small>
        </div>
      </header>

      {error && (
        <div className="error-banner" role="alert">
          {error}
          <button onClick={() => setError('')}>关闭</button>
        </div>
      )}

      <div className="motion-progress" role="tablist" aria-label="十动作">
        {data.registry.map((value, index) => {
          const row = data.matrix.find((item) => item.actionKey === value.key)!;
          return (
            <button
              role="tab"
              aria-selected={selected === value.key}
              key={value.key}
              className={selected === value.key ? 'active' : ''}
              onClick={() => selectAction(value.key)}
            >
              <span>{String(index + 1).padStart(2, '0')}</span>
              <b>{value.key}</b>
              <i>{row.ready ? '内容就绪' : `${row.approvedAssets}/${row.requiredAssets} 素材`}</i>
            </button>
          );
        })}
      </div>

      <div className="motion-tabs" role="tablist">
        {(
          [
            ['spec', 'MotionSpec'],
            ['bom', '素材 BOM'],
            ['prompt', 'AI 开发提示词'],
            ['import', '关键姿势导入与审批'],
          ] as const
        ).map(([id, label]) => (
          <button
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? 'active' : ''}
            key={id}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="motion-grid">
        <main>
          <section className="motion-card">
            <header>
              <div>
                <b>{selected}</b>
                <span>revision {spec.revision} · {definition.category}</span>
              </div>
              <em>
                {definition.loops
                  ? 'LOOP'
                  : definition.requiresHitFrame
                    ? 'ONE SHOT · EXACT 1 HIT'
                    : 'ONE SHOT'}
              </em>
            </header>

            {tab === 'spec' && (
              <div className="spec-panel">
                <label>
                  轮廓目标
                  <textarea
                    value={draft.silhouetteGoal}
                    onChange={(event) => setDraft({ ...draft, silhouetteGoal: event.target.value })}
                  />
                </label>
                <label>
                  武器意图
                  <textarea
                    value={draft.weaponIntent ?? ''}
                    disabled={!definition.requiresHitFrame && selected !== 'dash'}
                    onChange={(event) =>
                      setDraft({ ...draft, weaponIntent: event.target.value || null })
                    }
                  />
                </label>
                <div>
                  <label>
                    Duration tick
                    <input type="number" value={draft.durationTicks} readOnly />
                  </label>
                  <label>
                    Root motion
                    <select
                      value={draft.rootMotion}
                      onChange={(event) => setDraft({ ...draft, rootMotion: event.target.value })}
                    >
                      <option value="in-place">In Place</option>
                      <option value="preview-translation">Preview Translation</option>
                    </select>
                  </label>
                </div>
                <table className="phase-table">
                  <thead>
                    <tr><th>Pose/Phase</th><th>Start</th><th>End</th><th>意图</th></tr>
                  </thead>
                  <tbody>
                    {draft.phases.map((phase, index) => (
                      <tr key={phase.key}>
                        <td>{phase.key}</td><td>{phase.startTick}</td><td>{phase.endTick}</td>
                        <td>
                          <input
                            value={phase.intent}
                            onChange={(event) =>
                              setDraft({
                                ...draft,
                                phases: draft.phases.map((item, itemIndex) =>
                                  itemIndex === index ? { ...item, intent: event.target.value } : item,
                                ),
                              })
                            }
                          />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <button
                  className="primary"
                  disabled={busy || JSON.stringify(draft) === JSON.stringify(spec)}
                  onClick={() => void run('motion.spec.update', { spec: draft })}
                >
                  保存并重算 BOM / PromptPack
                </button>
              </div>
            )}

            {tab === 'bom' && (
              <ul className="bom-list">
                {assets.map((asset) => (
                  <li key={asset.assetSpecId}>
                    <b>{asset.actionKey}/{asset.poseKey}</b>
                    <span>{asset.purpose}</span>
                    <em>{asset.required ? 'REQUIRED' : 'OPTIONAL'}</em>
                    <i>{asset.state.toUpperCase()}</i>
                  </li>
                ))}
              </ul>
            )}

            {tab === 'prompt' && (
              <div className="prompt-entry-list">
                {prompts.map((entry) => (
                  <article key={entry.assetSpecId}>
                    <header><b>{entry.poseKey}</b><code>{entry.assetSpecId}</code></header>
                    <label>
                      正向提示词
                      <textarea value={entry.positive} readOnly onFocus={(event) => event.currentTarget.select()} />
                    </label>
                    <label>
                      负向提示词
                      <textarea className="negative" value={entry.negative} readOnly onFocus={(event) => event.currentTarget.select()} />
                    </label>
                  </article>
                ))}
              </div>
            )}

            {tab === 'import' && (
              <div className="asset-import-list">
                {assets.map((asset) => {
                  const binding = data.content.keyPoseBindings.find(
                    (value) => value.assetSpecId === asset.assetSpecId,
                  );
                  const pending = staged[asset.assetSpecId];
                  const review = binding ? reviews[binding.bindingId] : undefined;
                  const reviewedCurrentBinding = Boolean(
                    project &&
                      binding &&
                      review &&
                      review.projectId === project.projectId &&
                      review.projectRevision === project.revision &&
                      review.binding.bindingId === binding.bindingId &&
                      review.binding.sourceSha256 === binding.sourceSha256,
                  );
                  return (
                    <article className="key-pose-review-card" key={asset.assetSpecId}>
                      <div className="key-pose-review-content">
                        <div className="key-pose-review-meta">
                          <b>{asset.poseKey}</b>
                          <span>{asset.state.toUpperCase()}</span>
                          {binding && (
                            <code>{binding.sourceSha256.slice(0, 12)}… · {binding.width}×{binding.height}</code>
                          )}
                          {pending && <code>{pending.label}</code>}
                        </div>
                        {binding && !pending && (
                          <KeyPoseAlignmentEditor
                            binding={binding}
                            busy={busy}
                            onSave={(groundYMilliPx, scalePpm) =>
                              void run('motion.keyPose.alignment.set', {
                                bindingId: binding.bindingId,
                                expectedBindingRevision: binding.revision,
                                groundYMilliPx,
                                scalePpm,
                              })
                            }
                          />
                        )}
                        {review && binding && (
                          <figure className="key-pose-preview">
                            <img
                              src={review.safePreviewDataUrl}
                              alt={`${binding.actionKey} / ${binding.poseKey} 关键姿势安全预览`}
                              style={{
                                transform: `translateY(${binding.groundYMilliPx / 1000}px) scale(${binding.scalePpm / 1_000_000})`,
                              }}
                            />
                            <figcaption>
                              已从当前 CAS 绑定生成 256 px 有界预览
                              <code>{review.authority}</code>
                            </figcaption>
                          </figure>
                        )}
                      </div>
                      <div className="key-pose-review-actions">
                        <button className="secondary" disabled={busy} onClick={() => void choose(asset)}>
                          选择本地图片
                        </button>
                        {pending && (
                          <button
                            className="primary"
                            disabled={busy}
                            onClick={() => void run('motion.keyPose.promote', { stagingToken: pending.token })}
                          >
                            提升为 CAS 候选
                          </button>
                        )}
                        {binding && asset.state !== 'approved' && !pending && (
                          <>
                            <button className="secondary" disabled={busy} onClick={() => void preview(binding)}>
                              {reviewedCurrentBinding ? '重新读取并预览' : '读取 CAS 并预览'}
                            </button>
                            <button
                              className="primary"
                              disabled={busy || !reviewedCurrentBinding}
                              onClick={() => void approve(binding)}
                            >
                              审批已预览图片
                            </button>
                            {!reviewedCurrentBinding && (
                              <small>审批已锁定：必须先查看当前绑定的真实预览。</small>
                            )}
                          </>
                        )}
                        {asset.state === 'approved' && <small className="approved-note">该绑定已人工审批</small>}
                      </div>
                    </article>
                  );
                })}
              </div>
            )}
          </section>
        </main>

        <aside>
          <section>
            <b>真实表现策略</b>
            <dl>
              {strategies.map((value) => (
                <div key={value.part}><dt>{value.part}</dt><dd>{value.strategy}</dd></div>
              ))}
            </dl>
            <p>{strategies[0]?.ruleId}<br />{strategies[0]?.capabilityId}</p>
          </section>
          <section>
            <b>内容完整性</b>
            <ul>
              <li><i className={matrix.specReady ? 'ok' : ''}>{matrix.specReady ? '✓' : '○'}</i>结构化 MotionSpec</li>
              <li><i className={matrix.promptReady ? 'ok' : ''}>{matrix.promptReady ? '✓' : '○'}</i>PromptPack 条目</li>
              <li><i className={matrix.ready ? 'ok' : ''}>{matrix.ready ? '✓' : '○'}</i>{matrix.approvedAssets}/{matrix.requiredAssets} 图片已人工审核</li>
            </ul>
          </section>
          <section className="offline-note">
            <b>本地边界</b>
            <p>提示词合成是 Rust 纯函数，绑定 StyleSpec 内容哈希和 MotionSpec 哈希；不保存 provider 密钥。</p>
          </section>
        </aside>
      </div>
    </section>
  );
}

function KeyPoseAlignmentEditor({binding,busy,onSave}:{binding:KeyPoseBinding;busy:boolean;onSave:(groundYMilliPx:number,scalePpm:number)=>void}) {
  const [groundDraft,setGroundDraft]=useState(String(binding.groundYMilliPx/1000));
  const [scaleDraft,setScaleDraft]=useState(String(binding.scalePpm/1_000_000));
  useEffect(()=>{setGroundDraft(String(binding.groundYMilliPx/1000));setScaleDraft(String(binding.scalePpm/1_000_000))},[binding.bindingId,binding.revision,binding.groundYMilliPx,binding.scalePpm]);
  const ground=Number(groundDraft);const scale=Number(scaleDraft);
  const valid=Number.isFinite(ground)&&Math.abs(ground)<=100_000&&Number.isFinite(scale)&&scale>=0.01&&scale<=100;
  const groundYMilliPx=Math.round(ground*1000);const scalePpm=Math.round(scale*1_000_000);
  const dirty=valid&&(groundYMilliPx!==binding.groundYMilliPx||scalePpm!==binding.scalePpm);
  return <div className="key-pose-alignment"><label>Ground Y (px)<input type="number" min={-100000} max={100000} step={0.1} value={groundDraft} aria-invalid={!valid} disabled={busy} onChange={event=>setGroundDraft(event.target.value)}/></label><label>Scale ×<input type="number" min={0.01} max={100} step={0.001} value={scaleDraft} aria-invalid={!valid} disabled={busy} onChange={event=>setScaleDraft(event.target.value)}/></label><button className="secondary" disabled={busy||!dirty} onClick={()=>onSave(groundYMilliPx,scalePpm)}>保存对齐并重新审核</button></div>;
}
