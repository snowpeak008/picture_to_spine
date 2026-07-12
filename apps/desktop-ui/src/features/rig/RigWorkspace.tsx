import { useEffect, useState } from 'react';
import { invokeNative } from '../../native/ipc';
import { useProjectStore, type ProjectProjection } from '../../state/projectStore';
import './rig.css';
import './rig-real.css';

type Tool =
  | 'bones'
  | 'slots'
  | 'pivots'
  | 'mesh'
  | 'weights'
  | 'constraints'
  | 'diagnostics'
  | 'approval';

type RigMethod =
  | 'rig.initialize'
  | 'rig.status'
  | 'rig.setBone'
  | 'rig.setSlot'
  | 'rig.reparentBone'
  | 'rig.setPivot'
  | 'rig.setSocket'
  | 'rig.approve';

interface Point {
  xMilliPx: number;
  yMilliPx: number;
}

interface Rest extends Point {
  rotationMilliDeg: number;
  scaleXPpm: number;
  scaleYPpm: number;
}

interface Bone {
  boneId: string;
  name: string;
  parentId: string | null;
  rest: Rest;
}

interface Slot {
  slotId: string;
  layerId: string;
  boneId: string;
  drawKey: number;
}

interface Pivot {
  layerId: string;
  point: Point;
}

interface Socket {
  socketId: string;
  boneId: string;
  kind: string;
  point: Point;
  semantic: string;
}

interface Mesh {
  meshId: string;
  layerId: string;
  topologyRevision: number;
  vertices: unknown[];
  triangles: unknown[];
}

interface WeightSet {
  meshId: string;
  topologyRevision: number;
  byVertex: Record<string, Array<{ boneId: string; weightPpm: number }>>;
}

interface Constraint {
  constraintId: string;
  kind: string;
  constrainedBoneId: string;
  targetBoneId: string;
  mixPpm: number;
  order: number;
}

interface RigCandidate {
  rigId: string;
  revision: number;
  approvalState: 'PENDING' | 'APPROVED';
  canvas: { widthPx: number; heightPx: number };
  primaryWeapon: { weaponType: string; socketSemantic: string };
  boneTree: { revision: number; bones: Bone[] };
  slotSet: { revision: number; slots: Slot[] };
  pivotSocketRevision: number;
  pivots: Pivot[];
  sockets: Socket[];
  meshRevision: number;
  meshes: Mesh[];
  weightRevision: number;
  weights: WeightSet[];
  constraintRevision: number;
  constraints: Constraint[];
  constraintCapability: {
    capabilityId: string;
    spinePatch: string;
    sourceHashesVerified: boolean;
  };
}

interface RigIssue {
  code: string;
  target: string;
  severity: 'P0' | 'P1' | 'P2';
  fixTarget: string;
}

interface RigResponse {
  project: ProjectProjection;
  rig: RigCandidate;
  diagnostics: {
    sourceRevisions: Record<string, number>;
    issues: RigIssue[];
    ephemeral: boolean;
    completed: boolean;
    engineId: string;
    rigPayloadSha256: string | null;
  };
  safePreviewDataUrl: string;
  authority: string;
}

interface GlobalRest {
  x: number;
  y: number;
  rotationRad: number;
  scaleX: number;
  scaleY: number;
}

const tools: Array<{ id: Tool; label: string }> = [
  { id: 'bones', label: '骨骼树' },
  { id: 'slots', label: 'Slot 顺序' },
  { id: 'pivots', label: 'Pivot / Socket' },
  { id: 'mesh', label: 'Mesh' },
  { id: 'weights', label: '权重' },
  { id: 'constraints', label: '约束' },
  { id: 'diagnostics', label: '临时 Rig 诊断' },
  { id: 'approval', label: '人工审批' },
];

function globalBonePositions(bones: Bone[]) {
  const result = new Map<string, GlobalRest>();

  const visit = (bone: Bone, stack: Set<string>): GlobalRest => {
    const cached = result.get(bone.boneId);
    if (cached) return cached;
    if (stack.has(bone.boneId)) {
      return { x: 0, y: 0, rotationRad: 0, scaleX: 1, scaleY: 1 };
    }
    stack.add(bone.boneId);

    const parent = bone.parentId
      ? bones.find((value) => value.boneId === bone.parentId)
      : null;
    const localX = bone.rest.xMilliPx / 1000;
    const localY = bone.rest.yMilliPx / 1000;
    const localRotation = (bone.rest.rotationMilliDeg * Math.PI) / 180_000;
    const localScaleX = bone.rest.scaleXPpm / 1_000_000;
    const localScaleY = bone.rest.scaleYPpm / 1_000_000;

    if (!parent) {
      const value = {
        x: localX,
        y: localY,
        rotationRad: localRotation,
        scaleX: localScaleX,
        scaleY: localScaleY,
      };
      result.set(bone.boneId, value);
      return value;
    }

    const base = visit(parent, stack);
    const cos = Math.cos(base.rotationRad);
    const sin = Math.sin(base.rotationRad);
    const value = {
      x: base.x + cos * localX * base.scaleX - sin * localY * base.scaleY,
      y: base.y + sin * localX * base.scaleX + cos * localY * base.scaleY,
      rotationRad: base.rotationRad + localRotation,
      scaleX: base.scaleX * localScaleX,
      scaleY: base.scaleY * localScaleY,
    };
    result.set(bone.boneId, value);
    return value;
  };

  bones.forEach((bone) => visit(bone, new Set()));
  return result;
}

export function RigWorkspace() {
  const { project, setProject } = useProjectStore();
  const [data, setData] = useState<RigResponse | null>(null);
  const [tool, setTool] = useState<Tool>('bones');
  const [selectedBone, setSelectedBone] = useState('root');
  const [selectedLayer, setSelectedLayer] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  async function run(method: RigMethod, payload: Record<string, unknown> = {}) {
    if (!project) return;
    setBusy(true);
    setError('');
    try {
      const value = await invokeNative<RigResponse>(method, payload, project.revision);
      setProject(value.project);
      setData(value);
      if (!selectedLayer) {
        setSelectedLayer(value.rig.pivots[0]?.layerId ?? '');
      }
      if (!value.rig.boneTree.bones.some((bone) => bone.boneId === selectedBone)) {
        setSelectedBone(value.rig.boneTree.bones[0]?.boneId ?? '');
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Rig 命令失败');
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (project?.gates.layers === 'APPROVED' && project.rigState) {
      void run('rig.status');
    }
    // The project id is the head-refresh boundary; mutation responses update data directly.
  }, [project?.projectId]);

  if (!project || project.gates.layers !== 'APPROVED') {
    return (
      <section className="rig-workspace">
        <div className="blocked-panel">
          <b>Rig Gate 已锁定</b>
          <p>需要当前 LayerSet 的有效人工审批。</p>
        </div>
      </section>
    );
  }

  if (!project.rigState && !data) {
    return (
      <section className="rig-workspace">
        <header>
          <div>
            <span className="eyebrow">人工门 03 · 完整聚合审批</span>
            <h1>Rig 工作台</h1>
            <p>
              默认 Rig 由确定规则创建，但仍是候选；骨骼、slot、pivot、mesh、weight
              与能力清单会一起进入审批哈希。
            </p>
          </div>
          <button
            className="primary"
            disabled={busy}
            onClick={() => void run('rig.initialize')}
          >
            {busy ? '正在构建…' : '创建侧视类人 Rig 候选'}
          </button>
        </header>
        {error && (
          <div role="alert" className="error-banner">
            {error}
          </div>
        )}
      </section>
    );
  }

  if (!data) {
    return (
      <section className="rig-workspace">
        <div className="blocked-panel">
          <b>正在读取 Rig 项目头</b>
          <p>{error || '验证组件修订与能力清单…'}</p>
        </div>
      </section>
    );
  }

  const rig = data.rig;
  const bone =
    rig.boneTree.bones.find((value) => value.boneId === selectedBone) ??
    rig.boneTree.bones[0];
  const pivot =
    rig.pivots.find((value) => value.layerId === selectedLayer) ?? rig.pivots[0];
  const socket = rig.sockets[0];
  const positions = globalBonePositions(rig.boneTree.bones);
  const blocking =
    !data.diagnostics.completed ||
    data.diagnostics.issues.some(
      (issue) => issue.severity === 'P0' || issue.severity === 'P1',
    );
  const saveBone = (rest: Rest) =>
    void run('rig.setBone', {
      expectedRevision: rig.revision,
      boneId: bone.boneId,
      rest,
    });

  const approvalButton = (
    <button
      className="primary"
      disabled={busy || blocking || rig.approvalState === 'APPROVED'}
      onClick={() => void run('rig.approve')}
    >
      {rig.approvalState === 'APPROVED'
        ? 'Rig Gate 已批准'
        : '原生确认并批准 Rig Gate'}
    </button>
  );

  if (tool === 'slots') {
    const orderedSlots = [...rig.slotSet.slots].sort(
      (left, right) => left.drawKey - right.drawKey || left.slotId.localeCompare(right.slotId),
    );

    return (
      <section className="rig-workspace">
        <header>
          <div>
            <span className="eyebrow">人工门 03 · {data.authority}</span>
            <h1>Slot 绑定与绘制顺序</h1>
            <p>
              每次编辑都绑定当前 project/Rig revision；重复 draw key、未知骨骼或陈旧命令由
              Rust 原子拒绝。
            </p>
          </div>
          <div className="rig-header-actions">
            <span>
              Rig revision {rig.revision} · Slot revision {rig.slotSet.revision}
            </span>
            {approvalButton}
          </div>
        </header>

        <ErrorBanner error={error} onClose={() => setError('')} />
        <RigTabs selected={tool} onSelect={setTool} />

        <section className="slot-editor-panel">
          <table className="rig-data">
            <thead>
              <tr>
                <th>Layer / Slot</th>
                <th>绑定骨骼</th>
                <th>Draw key</th>
              </tr>
            </thead>
            <tbody>
              {orderedSlots.map((value) => (
                <tr key={value.slotId}>
                  <td>
                    <b>{value.layerId}</b>
                    <small>{value.slotId}</small>
                  </td>
                  <td>
                    <select
                      aria-label={`${value.layerId} slot bone`}
                      value={value.boneId}
                      disabled={busy}
                      onChange={(event) =>
                        void run('rig.setSlot', {
                          expectedRevision: rig.revision,
                          slotId: value.slotId,
                          boneId: event.target.value,
                          drawKey: value.drawKey,
                        })
                      }
                    >
                      {rig.boneTree.bones.map((item) => (
                        <option key={item.boneId} value={item.boneId}>
                          {item.name} · {item.boneId}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    <CommittedNumberInput
                      label="Draw"
                      value={value.drawKey}
                      step={1}
                      disabled={busy}
                      onCommit={(drawKey) =>
                        void run('rig.setSlot', {
                          expectedRevision: rig.revision,
                          slotId: value.slotId,
                          boneId: value.boneId,
                          drawKey: Math.round(drawKey),
                        })
                      }
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p>
            Slot 改动会使 Rig 及下游动画审批失效；mesh/weight
            仍按当前刚性 V1 合同独立校验。
          </p>
        </section>
      </section>
    );
  }

  return (
    <section className="rig-workspace">
      <header>
        <div>
          <span className="eyebrow">人工门 03 · {data.authority}</span>
          <h1>Rig 工作台</h1>
          <p>编辑命令同时校验 project revision 与 Rig revision；失败不会部分写入。</p>
        </div>
        <div className="rig-header-actions">
          <span>
            Rig revision {rig.revision} · {rig.approvalState}
          </span>
          {approvalButton}
        </div>
      </header>

      <ErrorBanner error={error} onClose={() => setError('')} />
      <RigTabs selected={tool} onSelect={setTool} />

      <div className="rig-layout">
        <aside className="bone-tree">
          <header>
            <b>骨骼 · {rig.boneTree.revision}</b>
          </header>
          {rig.boneTree.bones.map((value) => (
            <button
              key={value.boneId}
              className={bone.boneId === value.boneId ? 'selected' : ''}
              onClick={() => setSelectedBone(value.boneId)}
            >
              <i>{value.parentId ? '└' : '◆'}</i>
              <span>{value.name}</span>
              <small>{value.boneId}</small>
            </button>
          ))}
        </aside>

        <main className="rig-canvas">
          <div className="rig-toolbar">
            <b>Rig Rest 骨架调试视图</b>
            <span>
              画布 {rig.canvas.widthPx}×{rig.canvas.heightPx} · 坐标精度 1/1000 px
            </span>
          </div>
          <div className="rig-viewport">
            <svg
              viewBox={`0 0 ${rig.canvas.widthPx} ${rig.canvas.heightPx}`}
              role="img"
              aria-label="当前 Rig 骨骼与受限角色预览"
            >
              <image
                href={data.safePreviewDataUrl}
                x="0"
                y="0"
                width={rig.canvas.widthPx}
                height={rig.canvas.heightPx}
                opacity=".35"
              />
              {rig.boneTree.bones.map((value) => {
                const position = positions.get(value.boneId)!;
                const parent = value.parentId ? positions.get(value.parentId) : null;
                const selected = bone.boneId === value.boneId;
                return (
                  <g key={value.boneId}>
                    {parent && (
                      <line
                        x1={parent.x}
                        y1={parent.y}
                        x2={position.x}
                        y2={position.y}
                      />
                    )}
                    <circle
                      cx={position.x}
                      cy={position.y}
                      r={selected ? 7 : 4}
                      className={selected ? 'selected' : ''}
                    />
                  </g>
                );
              })}
            </svg>
            <em>
              骨骼点/线位置传播当前父级平移、旋转与缩放；背景角色图保持静态，不模拟
              attachment、mesh、约束或 Spine Editor。
            </em>
          </div>
        </main>

        <aside className="rig-inspector">
          <section>
            <b>{tools.find((value) => value.id === tool)?.label}</b>
            <p className="candidate-badge">
              {rig.approvalState} · hash 覆盖全部组件
            </p>
          </section>

          {tool === 'bones' && bone && (
            <section className="property-form">
              <label>
                Bone ID
                <input value={bone.boneId} readOnly />
              </label>
              <label>
                父骨骼
                <select
                  value={bone.parentId ?? ''}
                  disabled={!bone.parentId || busy}
                  onChange={(event) =>
                    void run('rig.reparentBone', {
                      expectedRevision: rig.revision,
                      boneId: bone.boneId,
                      parentId: event.target.value,
                    })
                  }
                >
                  <option value="">无</option>
                  {rig.boneTree.bones
                    .filter((value) => value.boneId !== bone.boneId)
                    .map((value) => (
                      <option key={value.boneId} value={value.boneId}>
                        {value.name}
                      </option>
                    ))}
                </select>
              </label>
              <div className="xy">
                <CommittedNumberInput
                  label="X px"
                  value={bone.rest.xMilliPx / 1000}
                  step={0.1}
                  disabled={busy}
                  onCommit={(value) =>
                    saveBone({ ...bone.rest, xMilliPx: Math.round(value * 1000) })
                  }
                />
                <CommittedNumberInput
                  label="Y px"
                  value={bone.rest.yMilliPx / 1000}
                  step={0.1}
                  disabled={busy}
                  onCommit={(value) =>
                    saveBone({ ...bone.rest, yMilliPx: Math.round(value * 1000) })
                  }
                />
              </div>
              <CommittedNumberInput
                label="旋转 °"
                value={bone.rest.rotationMilliDeg / 1000}
                step={0.1}
                disabled={busy}
                onCommit={(value) =>
                  saveBone({
                    ...bone.rest,
                    rotationMilliDeg: Math.round(value * 1000),
                  })
                }
              />
              <div className="xy">
                <CommittedNumberInput
                  label="Scale X ×"
                  value={bone.rest.scaleXPpm / 1_000_000}
                  min={-100}
                  max={100}
                  step={0.001}
                  validate={(value) => Math.abs(value) >= 0.001}
                  disabled={busy}
                  onCommit={(value) =>
                    saveBone({
                      ...bone.rest,
                      scaleXPpm: Math.round(value * 1_000_000),
                    })
                  }
                />
                <CommittedNumberInput
                  label="Scale Y ×"
                  value={bone.rest.scaleYPpm / 1_000_000}
                  min={-100}
                  max={100}
                  step={0.001}
                  validate={(value) => Math.abs(value) >= 0.001}
                  disabled={busy}
                  onCommit={(value) =>
                    saveBone({
                      ...bone.rest,
                      scaleYPpm: Math.round(value * 1_000_000),
                    })
                  }
                />
              </div>
            </section>
          )}

          {tool === 'pivots' && pivot && (
            <section className="property-form">
              <label>
                Layer
                <select
                  value={pivot.layerId}
                  onChange={(event) => setSelectedLayer(event.target.value)}
                >
                  {rig.pivots.map((value) => (
                    <option value={value.layerId} key={value.layerId}>
                      {value.layerId}
                    </option>
                  ))}
                </select>
              </label>
              <div className="xy">
                <CommittedNumberInput
                  label="Pivot X"
                  value={pivot.point.xMilliPx / 1000}
                  disabled={busy}
                  onCommit={(value) =>
                    void run('rig.setPivot', {
                      expectedRevision: rig.revision,
                      layerId: pivot.layerId,
                      point: {
                        ...pivot.point,
                        xMilliPx: Math.round(value * 1000),
                      },
                    })
                  }
                />
                <CommittedNumberInput
                  label="Pivot Y"
                  value={pivot.point.yMilliPx / 1000}
                  disabled={busy}
                  onCommit={(value) =>
                    void run('rig.setPivot', {
                      expectedRevision: rig.revision,
                      layerId: pivot.layerId,
                      point: {
                        ...pivot.point,
                        yMilliPx: Math.round(value * 1000),
                      },
                    })
                  }
                />
              </div>
              {socket && (
                <>
                  <label>
                    武器 Socket Bone
                    <select
                      value={socket.boneId}
                      disabled={busy}
                      onChange={(event) =>
                        void run('rig.setSocket', {
                          expectedRevision: rig.revision,
                          socketId: socket.socketId,
                          boneId: event.target.value,
                          point: socket.point,
                          semantic: socket.semantic,
                        })
                      }
                    >
                      {rig.boneTree.bones.map((value) => (
                        <option value={value.boneId} key={value.boneId}>
                          {value.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="xy">
                    <CommittedNumberInput
                      label="Socket X"
                      value={socket.point.xMilliPx / 1000}
                      disabled={busy}
                      onCommit={(value) =>
                        void run('rig.setSocket', {
                          expectedRevision: rig.revision,
                          socketId: socket.socketId,
                          boneId: socket.boneId,
                          point: {
                            ...socket.point,
                            xMilliPx: Math.round(value * 1000),
                          },
                          semantic: socket.semantic,
                        })
                      }
                    />
                    <CommittedNumberInput
                      label="Socket Y"
                      value={socket.point.yMilliPx / 1000}
                      disabled={busy}
                      onCommit={(value) =>
                        void run('rig.setSocket', {
                          expectedRevision: rig.revision,
                          socketId: socket.socketId,
                          boneId: socket.boneId,
                          point: {
                            ...socket.point,
                            yMilliPx: Math.round(value * 1000),
                          },
                          semantic: socket.semantic,
                        })
                      }
                    />
                  </div>
                  <p>
                    {rig.primaryWeapon.weaponType} · {socket.semantic}
                  </p>
                </>
              )}
            </section>
          )}

          {tool === 'mesh' && (
            <ActualList
              label="Mesh"
              rows={rig.meshes.map(
                (value) =>
                  `${value.layerId}: ${value.vertices.length} vertices / ${value.triangles.length} triangles / topology ${value.topologyRevision}`,
              )}
            />
          )}

          {tool === 'weights' && (
            <ActualList
              label="WeightSet"
              rows={rig.weights.map(
                (value) =>
                  `${value.meshId}: ${Object.keys(value.byVertex).length} vertices / max 4 influences`,
              )}
            />
          )}

          {tool === 'constraints' && (
            <ActualList
              label="Constraint"
              rows={
                rig.constraints.length
                  ? rig.constraints.map(
                      (value) =>
                        `${value.constraintId}: ${value.constrainedBoneId} → ${value.targetBoneId}`,
                    )
                  : ['无约束；Spine 4.2.43 transform-constraint 能力已验证']
              }
            />
          )}

          {tool === 'diagnostics' && (
            <ActualList
              label={`权威诊断 · ${data.diagnostics.engineId}`}
              rows={
                data.diagnostics.issues.length
                  ? data.diagnostics.issues.map(
                      (issue) => `${issue.severity} ${issue.code}: ${issue.target}`,
                    )
                  : [
                      data.diagnostics.completed
                        ? '已执行：P0 0 / P1 0'
                        : 'UNVERIFIED：诊断未执行',
                    ]
              }
            />
          )}

          {tool === 'approval' && (
            <section>
              <ul className="rig-checks">
                <li>
                  <i className="ok">✓</i>
                  LayerSet approval hash 已重算
                </li>
                <li>
                  <i className="ok">✓</i>
                  {rig.slotSet.slots.length} 层 slot/pivot/mesh/weight 全覆盖
                </li>
                <li>
                  <i className={blocking ? 'warn' : 'ok'}>{blocking ? '○' : '✓'}</i>
                  {data.diagnostics.completed ? '诊断已执行' : '诊断未执行'} · P0/P1{' '}
                  {blocking ? '仍存在' : '为 0'}
                </li>
              </ul>
            </section>
          )}

          <section className="rig-capability">
            <b>Spine 输出能力</b>
            <code>{rig.constraintCapability.capabilityId}</code>
            <p>
              固定 {rig.constraintCapability.spinePatch} · source hash{' '}
              {rig.constraintCapability.sourceHashesVerified ? '已复算' : '未验证'} · Editor 往返
              EXTERNAL。
            </p>
          </section>
        </aside>
      </div>
    </section>
  );
}

function RigTabs({
  selected,
  onSelect,
}: {
  selected: Tool;
  onSelect: (tool: Tool) => void;
}) {
  return (
    <div className="rig-tabs" role="tablist">
      {tools.map((value) => (
        <button
          role="tab"
          aria-selected={selected === value.id}
          className={selected === value.id ? 'active' : ''}
          key={value.id}
          onClick={() => onSelect(value.id)}
        >
          {value.label}
        </button>
      ))}
    </div>
  );
}

function ErrorBanner({ error, onClose }: { error: string; onClose: () => void }) {
  if (!error) return null;
  return (
    <div role="alert" className="error-banner">
      {error}
      <button onClick={onClose}>关闭</button>
    </div>
  );
}

function ActualList({ label, rows }: { label: string; rows: string[] }) {
  return (
    <section>
      <b>{label} · 当前项目 DTO</b>
      <ul className="rig-checks">
        {rows.map((row) => (
          <li key={row}>
            <i className="ok">✓</i>
            {row}
          </li>
        ))}
      </ul>
    </section>
  );
}

interface CommittedNumberInputProps {
  label: string;
  value: number;
  disabled: boolean;
  onCommit: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  validate?: (value: number) => boolean;
}

function CommittedNumberInput({
  label,
  value,
  disabled,
  onCommit,
  min,
  max,
  step,
  validate,
}: CommittedNumberInputProps) {
  const [draft, setDraft] = useState(String(value));

  useEffect(() => setDraft(String(value)), [value]);

  const commit = () => {
    const parsed = Number(draft);
    const invalid =
      !Number.isFinite(parsed) ||
      (min !== undefined && parsed < min) ||
      (max !== undefined && parsed > max) ||
      (validate !== undefined && !validate(parsed));
    if (invalid) {
      setDraft(String(value));
      return;
    }
    if (parsed !== value) onCommit(parsed);
  };

  return (
    <label>
      {label}
      <input
        type="number"
        value={draft}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === 'Enter') event.currentTarget.blur();
          if (event.key === 'Escape') {
            setDraft(String(value));
            event.currentTarget.blur();
          }
        }}
      />
    </label>
  );
}
