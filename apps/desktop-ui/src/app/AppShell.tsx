import { useEffect, useState } from 'react';
import { DiagnosticsPage } from '../features/diagnostics/DiagnosticsPage';
import { AnimationWorkspace } from '../features/animation/AnimationWorkspace';
import { LayerWorkspace } from '../features/layers/LayerWorkspace';
import { ExportWorkspace } from '../features/export/ExportWorkspace';
import { ImportWorkspace } from '../features/m03/feature';
import { MotionWorkspace } from '../features/motion/MotionWorkspace';
import { RigWorkspace } from '../features/rig/RigWorkspace';
import { RemoteGpuSettings } from '../features/settings/RemoteGpuSettings';
import { SpineCliSettings } from '../features/settings/SpineCliSettings';
import { invokeNative, nativeAvailable } from '../native/ipc';
import { useProjectStore, type ProjectProjection } from '../state/projectStore';
import './project.css';
import './recent.css';

type Stage = 'overview' | 'master' | 'spec' | 'layers' | 'rig' | 'animation' | 'review' | 'export' | 'settings' | 'diagnostics';
const stages: Array<{ id: Stage; icon: string; label: string; gate?: boolean }> = [
  { id: 'overview', icon: '◫', label: '项目概览' },
  { id: 'master', icon: '●', label: '输入与母版', gate: true },
  { id: 'spec', icon: '◆', label: '规格与提示词' },
  { id: 'layers', icon: '▤', label: '分层与素材', gate: true },
  { id: 'rig', icon: '⌘', label: 'Rig 工作台', gate: true },
  { id: 'animation', icon: '▶', label: '动画工作台', gate: true },
  { id: 'review', icon: '✓', label: '动作审核', gate: true },
  { id: 'export', icon: '⇧', label: '导出与验证' },
  { id: 'settings', icon: '⚙', label: '外部工具设置' },
  { id: 'diagnostics', icon: '⚠', label: '环境诊断' },
];
const actionKeys = ['idle','run','jump','fall','dash','attack_01','attack_02','attack_03','hit','death'];

function Overview({ createProject, openProject, native }: { createProject: (name:string) => Promise<void>; openProject: (projectId:string) => Promise<void>; native:boolean }) {
  const [name,setName]=useState('');const[busy,setBusy]=useState(false);const[error,setError]=useState('');const[recent,setRecent]=useState<ProjectProjection[]>([]);
  const project=useProjectStore(state=>state.project);
  useEffect(()=>{if(native)void invokeNative<ProjectProjection[]>('project.recent').then(setRecent).catch(error=>setError(error instanceof Error?error.message:'最近项目读取失败'));},[native]);
  async function create(){setBusy(true);setError('');try{await createProject(name)}catch(e){setError(e instanceof Error?e.message:'创建项目失败')}finally{setBusy(false)}}
  async function open(projectId:string){setBusy(true);setError('');try{await openProject(projectId)}catch(e){setError(e instanceof Error?e.message:'打开项目失败')}finally{setBusy(false)}}
  return <>
    <section className="hero-panel"><div><span className="eyebrow">PRODUCTION ASSIST CORE · RELEASE 未授权</span><h1>角色 Spine 动画资产生产辅助工具</h1><p>项目事实源、图片 CAS、五组人工门、十动作时间轴和开放格式不可变导出均由 Native 宿主校验；外部 Spine Editor 与私有 GPU 仍保持独立状态。</p></div><div className="new-project-inline"><input value={name} maxLength={120} placeholder="项目名称" onChange={e=>setName(e.target.value)}/><button className="primary" disabled={!native||!name.trim()||busy} onClick={()=>void create()}>{busy?'正在创建…':'创建本地项目'}</button></div></section>{error&&<div className="error-banner">{error}</div>}
    <section className="gate-grid">
      <article className={`gate-card ${project?.gates.master!=='APPROVED'?'current':''}`}><header><span>01</span><b>批准母版</b><em>{project?.gates.master??'等待项目'}</em></header><p>导入本地 PNG、JPEG 或 WebP，确认风格、视角、角色身份与唯一主武器。</p><div className="empty-preview"><span>{project?.gates.master==='APPROVED'?'✓':'＋'}</span><strong>{project?.gates.master==='APPROVED'?'当前母版审批有效':'导入角色图片'}</strong><small>不会自动上传或调用公网生图</small></div></article>
      <article className={`gate-card ${project?.gates.master==='APPROVED'&&project.gates.rig!=='APPROVED'?'current':''}`}><header><span>02</span><b>素材与 Rig</b><em>{project?`${project.gates.layers} / ${project.gates.rig}`:'等待上游'}</em></header><p>Rust 重放遮罩笔划，并从 CAS 重算重组 QA；Rig 的骨骼、slot、pivot、mesh、weight 与能力清单进入同一审批哈希。</p><ul><li>分层审批 <i className={project?.gates.layers==='APPROVED'?'ok':''}>{project?.gates.layers??'LOCKED'}</i></li><li>骨骼审批 <i className={project?.gates.rig==='APPROVED'?'ok':''}>{project?.gates.rig??'LOCKED'}</i></li><li>Spine 4.2.43 静态合同 <i className="ok">CONTRACT_VERIFIED</i></li></ul></article>
      <article className={`gate-card ${project?.gates.rig==='APPROVED'&&project.gates.hits!=='APPROVED'?'current':''}`}><header><span>03</span><b>动作与命中帧</b><em>{project?`${project.poseApprovalCount}/10 Pose · ${project.hitApprovalCount}/3 Hit`:'等待上游'}</em></header><p>固定十个动作；三个攻击动作分别审核关键姿势和唯一 hit frame。</p><div className="action-pills">{actionKeys.map(key=><span key={key}>{key}</span>)}</div></article>
    </section>
    {recent.length>0&&<section className="recent-projects"><header><b>最近本地项目</b><span>逐项校验本机项目头</span></header>{recent.map(item=>{const unavailable=item.availability!=='AVAILABLE';return <button key={item.projectId} disabled={busy||unavailable} className={unavailable?'unavailable':''} onClick={()=>void open(item.projectId)}><strong>{item.displayName}</strong><span>{unavailable?item.diagnosticCode:`revision ${item.revision} · ${item.workflowStage}`}</span><em>{unavailable?'已隔离':`${item.gates.master}/${item.gates.layers}`}</em></button>})}</section>}
    <section className="bottom-grid"><article><header><b>当前能力</b><span>权威状态</span></header><dl><div><dt>Native 图片接收</dt><dd className={native?'good':''}>{native?'已接通':'不可用'}</dd></div><div><dt>开放格式导出</dt><dd className="good">权威预检 / 不可覆盖提交</dd></div><div><dt>AppContainer AI Worker</dt><dd>UNVERIFIED_EXCLUDED</dd></div><div><dt>Spine Editor 4.2.43</dt><dd>EXTERNAL</dd></div></dl></article><article><header><b>项目边界</b><span>固定 V1</span></header><p className="boundary-copy">二次元类人 · 横版侧视 · 一件已确认主武器<br/>只生产角色动画资产，不生产重度动作游戏逻辑</p></article></section>
  </>;
}

export function AppShell() {
  const [stage, setStage] = useState<Stage>('overview');
  const native=nativeAvailable();
  const [ipcState,setIpcState]=useState(native?'CONNECTING':'UNAVAILABLE');
  const {project,setProject}=useProjectStore();
  useEffect(()=>{if(native)void invokeNative<{ipc:string;currentProject:ProjectProjection|null}>('bootstrap.status').then(v=>{setIpcState(v.ipc);setProject(v.currentProject)}).catch(()=>setIpcState('FAILED'));},[native,setProject]);
  async function createProject(name:string){const value=await invokeNative<ProjectProjection>('project.create',{name});setProject(value);setStage('master')}
  async function openProject(projectId:string){const value=await invokeNative<ProjectProjection>('project.open',{projectId});setProject(value);setStage(value.gates.hits==='APPROVED'?'export':value.animationState==='PRESENT'?'animation':value.gates.rig==='APPROVED'?'spec':value.gates.layers==='APPROVED'?'rig':value.gates.master==='APPROVED'?'layers':'master')}
  let content = <Overview createProject={createProject} openProject={openProject} native={native} />;
  if (stage === 'master') content = <ImportWorkspace />;
  if (stage === 'layers') content = <LayerWorkspace />;
  if (stage === 'rig') content = <RigWorkspace />;
  if (stage === 'spec') content = <MotionWorkspace />;
  if (stage === 'animation' || stage === 'review') content = <AnimationWorkspace reviewMode={stage === 'review'} />;
  if (stage === 'export') content = <ExportWorkspace />;
  if (stage === 'settings') content = <div className="settings-stack"><SpineCliSettings /><RemoteGpuSettings /></div>;
  if (stage === 'diagnostics') content = <DiagnosticsPage />;
  const completedGates=[project?.gates.master,project?.gates.layers,project?.gates.rig,project?.gates.poses,project?.gates.hits].filter(value=>value==='APPROVED').length;
  return <div className="app-shell">
    <header className="titlebar"><div className="brand-mark">FS</div><div><strong>FlashToSpine</strong><span>PRODUCTION ASSIST CORE</span></div><div className="title-project">{project?.displayName??'未打开项目'} <em>{project?`revision ${project.revision}`:'未保存'}</em></div><div className="title-status"><span className="status-dot"/>IPC：{ipcState} · Release 未授权</div></header>
    <aside className="sidebar"><div className="workflow-title"><span>制作流程</span><small>{completedGates}/5 审批完成</small></div><nav>{stages.map(item=>{const allowed=item.id==='overview'||item.id==='settings'||item.id==='diagnostics'||(item.id==='master'&&project!==null)||(item.id==='layers'&&project?.gates.master==='APPROVED')||(item.id==='spec'&&project?.gates.master==='APPROVED')||(item.id==='rig'&&project?.gates.layers==='APPROVED')||((item.id==='animation'||item.id==='review')&&project?.gates.rig==='APPROVED'&&project.motionState==='PRESENT')||(item.id==='export'&&project!==null);const locked=!allowed;return <button key={item.id} disabled={locked} title={locked?'需要先创建项目并满足该工作台的上游人工门':''} className={stage===item.id?'active':''} onClick={()=>setStage(item.id)}><b>{item.icon}</b><span>{item.label}</span>{item.gate&&<i title="需要人工审批">●</i>}</button>})}</nav><div className="sidebar-foot"><p>目标格式</p><strong>Spine 4.2.43</strong><span>Editor 往返：EXTERNAL</span></div></aside>
    <main className="workspace">{content}</main>
    <footer className="statusbar"><span>● Core：IMPLEMENTED · EXTERNALS PENDING</span><span>外部能力：EXTERNAL / UNVERIFIED</span><span>默认本地模式 · 无公网请求</span></footer>
  </div>;
}
