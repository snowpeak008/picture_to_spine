import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';
import { invokeNative } from '../../native/ipc';
import {
  useProjectStore,
  type LayerProjection,
  type LayerSetProjection,
  type ProjectProjection,
} from '../../state/projectStore';
import './layers.css';

interface RecompositionMetrics {
  missingPixels: number;
  overlapPixels: number;
  changedVisiblePixels: number;
  alphaErrorPixels: number;
  emptyLayerMasks: number;
}

interface LayerResponse {
  project: ProjectProjection;
  layerSet: LayerSetProjection;
  metrics: RecompositionMetrics;
  approvalQaPassed: boolean;
  requiredRoles: string[];
  canvas: { width: number; height: number };
  safePreviewDataUrl: string;
  selectedLayerId: string;
  selectedLayerPreviewDataUrl: string;
  authority: 'RUST_CAS_RECOMPUTED';
}

interface LayerChooseResult { cancelled:boolean;stagingToken?:string;fileName?:string;report?:{mediaType:string;width:number;height:number;hasAlpha:boolean} }

interface StrokePoint {
  xMilli: number;
  yMilli: number;
  pressureMilli: number;
  tick: number;
}

const optionalRoles = ['accessory'];

export function LayerWorkspace() {
  const { project, setProject } = useProjectStore();
  const [data, setData] = useState<LayerResponse | null>(null);
  const [selected, setSelected] = useState<string | null>(project?.activeLayerSet?.layers[0]?.layerId ?? null);
  const [mode, setMode] = useState<'add' | 'subtract'>('add');
  const [radius, setRadius] = useState(18);
  const [draft, setDraft] = useState<StrokePoint[]>([]);
  const draftRef = useRef<StrokePoint[]>([]);
  const drawing = useRef(false);
  const boardRef = useRef<HTMLDivElement>(null);
  const [newLayerName, setNewLayerName] = useState('optional-detail');
  const [newLayerRole, setNewLayerRole] = useState('accessory');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [replacement, setReplacement] = useState<{ token:string; layerId:string; label:string } | null>(null);

  async function run(method: 'layers.initialize' | 'layers.add' | 'layers.delete' | 'layers.reorder' | 'layers.stroke' | 'layers.replacement.promote' | 'layers.status', payload: Record<string, unknown> = {}) {
    if (!project) return;
    setBusy(true);
    setError('');
    try {
      const response = await invokeNative<LayerResponse>(method, payload, project.revision);
      setProject(response.project);
      setData(response);
      setSelected(response.selectedLayerId);
      if (method === 'layers.replacement.promote') setReplacement(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '分层命令失败');
    } finally {
      setBusy(false);
    }
  }

  async function chooseReplacement(layer: LayerProjection) {
    if (!project || busy) return;
    setBusy(true);
    setError('');
    try {
      const result = await invokeNative<LayerChooseResult>(
        'layers.replacement.chooseAndPreflight',
        { layerId: layer.layerId },
        project.revision,
      );
      if (!result.cancelled && result.stagingToken) {
        setReplacement({
          token: result.stagingToken,
          layerId: layer.layerId,
          label: `${result.fileName ?? 'layer.png'} · ${result.report?.width}×${result.report?.height} · ${result.report?.hasAlpha ? 'Alpha' : '无 Alpha'}`,
        });
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '分层替换图片预检失败');
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (!project || project.gates.master !== 'APPROVED' || !project.activeLayerSet) return;
    void run('layers.status', selected ? { layerId: selected } : {});
    // Initial authority refresh is keyed to the project identity; later revisions are applied by run().
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project?.projectId]);

  async function chooseLayer(layer: LayerProjection) {
    if (!project || busy) return;
    setSelected(layer.layerId);
    await run('layers.status', { layerId: layer.layerId });
  }

  function pointFromEvent(event: ReactPointerEvent<HTMLDivElement>): StrokePoint | null {
    const board = boardRef.current;
    if (!board || !data) return null;
    const bounds = board.getBoundingClientRect();
    if (bounds.width <= 0 || bounds.height <= 0) return null;
    const x = Math.max(0, Math.min(data.canvas.width - 1, ((event.clientX - bounds.left) / bounds.width) * data.canvas.width));
    const y = Math.max(0, Math.min(data.canvas.height - 1, ((event.clientY - bounds.top) / bounds.height) * data.canvas.height));
    return {
      xMilli: Math.round(x * 1_000),
      yMilli: Math.round(y * 1_000),
      pressureMilli: Math.round((event.pressure > 0 ? event.pressure : 1) * 1_000),
      tick: Date.now(),
    };
  }

  function beginStroke(event: ReactPointerEvent<HTMLDivElement>) {
    if (!selected || busy || !data) return;
    const point = pointFromEvent(event);
    if (!point) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drawing.current = true;
    draftRef.current = [point];
    setDraft([point]);
  }

  function continueStroke(event: ReactPointerEvent<HTMLDivElement>) {
    if (!drawing.current || draftRef.current.length >= 5_000) return;
    const point = pointFromEvent(event);
    if (!point) return;
    const previous = draftRef.current.at(-1);
    if (previous && Math.abs(previous.xMilli - point.xMilli) + Math.abs(previous.yMilli - point.yMilli) < 1_000) return;
    draftRef.current = [...draftRef.current, point];
    setDraft(draftRef.current);
  }

  async function finishStroke(event: ReactPointerEvent<HTMLDivElement>) {
    if (!drawing.current || !data || !selected) return;
    drawing.current = false;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    const points = draftRef.current;
    const layer = data.layerSet.layers.find((value) => value.layerId === selected);
    setDraft([]);
    draftRef.current = [];
    if (!layer || points.length === 0) return;
    await run('layers.stroke', {
      layerId: layer.layerId,
      baseMaskSha256: layer.maskSha256,
      radiusMilli: radius * 1_000,
      mode,
      points,
    });
  }

  async function approve() {
    if (!project || !data) return;
    setBusy(true);
    setError('');
    try {
      const response = await invokeNative<{
        project: ProjectProjection;
        layerSet: LayerSetProjection;
        metrics: RecompositionMetrics;
        authority: 'RUST_CAS_RECOMPUTED';
      }>('layers.approve', {}, project.revision);
      setProject(response.project);
      setData({ ...data, ...response });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Layer Gate 审批失败');
    } finally {
      setBusy(false);
    }
  }

  async function moveSelected(offset: -1 | 1) {
    if (!selected || !data) return;
    const ids = data.layerSet.layers.map((layer) => layer.layerId);
    const index = ids.indexOf(selected);
    const destination = index + offset;
    if (index < 0 || destination < 0 || destination >= ids.length) return;
    [ids[index], ids[destination]] = [ids[destination], ids[index]];
    await run('layers.reorder', { layerIds: ids, selectedLayerId: selected });
  }

  if (!project || project.gates.master !== 'APPROVED') {
    return <section className="layer-workspace"><div className="blocked-panel"><b>Layer Gate 已锁定</b><p>需要当前项目的有效母版审批。</p></div></section>;
  }

  if (!project.activeLayerSet && !data) {
    return <section className="layer-workspace"><header><div><span className="eyebrow">人工门 02 · Native raster authority</span><h1>分层与素材修复</h1><p>标准清单包含 17 个 V1 必需语义层；空层、缺像素、重叠或 Alpha 误差均阻断审批。</p></div><button className="primary" disabled={busy} onClick={() => void run('layers.initialize')}>{busy ? '正在建立 CAS 素材…' : '创建标准分层清单'}</button></header>{error && <div className="error-banner">{error}</div>}</section>;
  }

  if (!data) {
    return <section className="layer-workspace"><div className="blocked-panel"><b>正在读取权威 LayerSet</b><p>{error || '从本地 CAS 重算重组质量…'}</p></div></section>;
  }

  const selectedLayer = data.layerSet.layers.find((layer) => layer.layerId === selected) ?? data.layerSet.layers[0];
  const passes = data.approvalQaPassed;
  const points = draft.map((point) => `${point.xMilli / 1_000},${point.yMilli / 1_000}`).join(' ');

  return <section className="layer-workspace">
    <header><div><span className="eyebrow">人工门 02 · {data.authority}</span><h1>分层与素材修复</h1><p>所有笔划由 Rust 按基准遮罩重放，附件、遮罩与项目头写入本地 CAS revision 链。</p></div><button className="primary" disabled={busy || !passes || data.layerSet.approvalState === 'APPROVED'} onClick={() => void approve()}>{data.layerSet.approvalState === 'APPROVED' ? 'Layer Gate 已批准' : busy ? '正在处理…' : '原生确认并批准 Layer Gate'}</button></header>
    {error && <div className="error-banner">{error}<button onClick={() => setError('')}>关闭</button></div>}
    <div className="layer-layout">
      <aside className="layer-tree"><header><b>LayerSet · revision {data.layerSet.revision}</b><em>{data.layerSet.approvalState}</em></header>{data.layerSet.layers.map((layer) => <button type="button" className={selectedLayer?.layerId === layer.layerId ? 'selected' : ''} key={layer.layerId} onClick={() => void chooseLayer(layer)} disabled={busy}><span className="visibility" aria-label={layer.visible ? '可见' : '隐藏'}>{layer.visible ? '◉' : '○'}</span><span>{layer.name}</span><i>{layer.role}</i></button>)}<div className="add-layer-form"><input value={newLayerName} maxLength={120} aria-label="新层名称" onChange={(event) => setNewLayerName(event.target.value)} /><select value={newLayerRole} aria-label="新层角色" onChange={(event) => setNewLayerRole(event.target.value)}>{optionalRoles.map((role) => <option value={role} key={role}>{role}</option>)}</select><button className="add-layer" disabled={busy || !newLayerName.trim()} onClick={() => void run('layers.add', { name: newLayerName, role: newLayerRole })}>＋ 添加可选手工层</button></div></aside>
      <main className="mask-editor"><div className="tool-strip"><button className={mode === 'add' ? 'active' : ''} onClick={() => setMode('add')}>加入当前层</button><button className={mode === 'subtract' ? 'active erase' : ''} onClick={() => setMode('subtract')}>从当前层移除</button><label>半径 <input type="range" min="1" max="128" value={radius} onChange={(event) => setRadius(Number(event.target.value))} /> {radius}px</label><span>当前：{selectedLayer?.name}</span></div><div className="layer-canvas"><div ref={boardRef} className={`raster-board ${busy ? 'busy' : ''}`} style={{ aspectRatio: `${data.canvas.width} / ${data.canvas.height}` }} onPointerDown={beginStroke} onPointerMove={continueStroke} onPointerUp={(event) => void finishStroke(event)} onPointerCancel={(event) => void finishStroke(event)}><img src={data.safePreviewDataUrl} alt="Native 生成的受限原图预览" className="source-raster" draggable={false}/><img src={data.selectedLayerPreviewDataUrl} alt="当前分层附件预览" className="selected-raster" draggable={false}/><svg viewBox={`0 0 ${data.canvas.width} ${data.canvas.height}`} aria-hidden="true"><polyline points={points} fill="none" stroke={mode === 'add' ? '#57e2d2' : '#ff7788'} strokeWidth={radius * 2} strokeLinecap="round" strokeLinejoin="round" /></svg></div><em>受限预览 {data.canvas.width}×{data.canvas.height} · 原图不进入 WebView 解码器</em></div></main>
      <aside className="layer-inspector"><section><b>当前素材</b><dl><div><dt>角色</dt><dd>{selectedLayer?.role}</dd></div><div><dt>遮罩</dt><dd>{selectedLayer?.maskSha256.slice(0, 10)}…</dd></div><div><dt>附件</dt><dd>{selectedLayer?.attachmentSha256.slice(0, 10)}…</dd></div></dl>{selectedLayer&&<div className="layer-replacement"><button className="secondary" disabled={busy} onClick={()=>void chooseReplacement(selectedLayer)}>选择全画布透明 PNG 替换层</button>{replacement?.layerId===selectedLayer.layerId&&<><small>{replacement.label}</small><button className="primary" disabled={busy} onClick={()=>void run('layers.replacement.promote',{stagingToken:replacement.token})}>导入附件并采用 Alpha 遮罩</button></>}</div>}<div className="layer-order-actions"><button disabled={busy || data.layerSet.layers[0]?.layerId === selected} onClick={() => void moveSelected(-1)}>上移</button><button disabled={busy || data.layerSet.layers.at(-1)?.layerId === selected} onClick={() => void moveSelected(1)}>下移</button><button className="danger" disabled={busy || selectedLayer?.role !== 'accessory'} onClick={() => selectedLayer && void run('layers.delete', { layerId: selectedLayer.layerId })}>删除可选层</button></div></section><section><b>权威重组 QA</b><ul><li>缺失像素 <i className={data.metrics.missingPixels === 0 ? 'ok' : ''}>{data.metrics.missingPixels}</i></li><li>接缝重叠 <i className={data.metrics.overlapPixels === 0 ? 'ok' : ''}>{data.metrics.overlapPixels}</i></li><li>相对原图变化 <i className={passes ? 'ok' : ''}>{data.metrics.changedVisiblePixels}</i></li><li>Alpha 错误 <i className={data.metrics.alphaErrorPixels === 0 ? 'ok' : ''}>{data.metrics.alphaErrorPixels}</i></li><li>空遮罩层 <i className={data.metrics.emptyLayerMasks === 0 ? 'ok' : ''}>{data.metrics.emptyLayerMasks}</i></li></ul><p className={passes ? 'qa-pass' : 'qa-block'}>{passes ? '结构 QA 与本地人工素材 provenance 通过，可请求审批。' : 'QA 未通过，审批按钮保持锁定。'}</p></section><section className="worker-off"><b>AI Worker 未包含</b><p>当前 Core 只接收并人工拆分图片。AppContainer Worker 保持 UNVERIFIED_EXCLUDED。</p></section></aside>
    </div>
  </section>;
}
