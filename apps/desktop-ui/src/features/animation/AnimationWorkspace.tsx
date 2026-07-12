import { useEffect, useState } from 'react';
import { invokeNative } from '../../native/ipc';
import { useProjectStore, type ProjectProjection } from '../../state/projectStore';
import { PixiRigPreview, type AttachmentPreviewProjection, type PreviewClip, type PreviewRig } from './PixiRigPreview';
import './animation.css';
import './animation-real.css';

interface Keyframe {
  keyframeId: string;
  tick: number;
  valuesMilli: number[];
  curve: 'stepped' | 'linear' | 'bezier';
  bezierMilli: [number,number,number,number] | null;
}
interface Track { trackId:string; targetId:string; channel:string; keyframes:Keyframe[] }
interface Clip extends PreviewClip { clipId:string; actionKey:string; revision:number; timeBase:{numerator:number;denominator:number}; tracks:Track[] }
interface PoseMarker { markerId:string; actionKey:string; poseKey:string; tick:number }
interface GameplayMarker { markerId:string; actionKey:string; kind:string; startTick:number; endTick:number; socketId:string|null }
interface AnimationSet { revision:number; approvedRigSha256:string; motionContentSha256:string; clips:Clip[]; reviewPoseMarkers:PoseMarker[]; gameplayMarkers:GameplayMarker[] }
interface Rig extends PreviewRig {
  rigId:string;
  revision:number;
  boneTree:{bones:Array<{boneId:string;name:string;parentId:string|null;rest:{xMilliPx:number;yMilliPx:number;rotationMilliDeg:number;scaleXPpm:number;scaleYPpm:number}}>};
  slotSet:{slots:Array<{slotId:string;layerId:string;boneId:string;drawKey:number}>};
  sockets:Array<{socketId:string;boneId:string;kind:string;semantic:string}>;
}
interface Motion {
  assets:Array<{actionKey:string;poseKey:string;state:string}>;
  specs:Array<{actionKey:string;phases:Array<{key:string;startTick:number;endTick:number}>}>;
}
interface ActionRow { definition:{key:string;category:string;loops:boolean;requiresHitFrame:boolean}; poseApproved:boolean; hitApproved:boolean }
interface Issue { code:string; target:string; severity:string; tick:number|null; explanation:string }
interface AnimationResponse { project:ProjectProjection; animation:AnimationSet; rig:Rig; motion:Motion; actions:ActionRow[]; diagnostics:Issue[]; attachmentPreview:AttachmentPreviewProjection; authority:string }

type AnimationMethod = 'animation.initialize'|'animation.status'|'animation.track.put'|'animation.poseMarker.set'|'animation.hitMarker.set'|'animation.pose.approve'|'animation.hit.approve';
type NewTrackChannel = 'bone-translate'|'bone-rotate'|'bone-scale'|'slot-color'|'draw-order';

export function AnimationWorkspace({reviewMode=false}:{reviewMode?:boolean}){
  const {project,setProject}=useProjectStore();
  const [data,setData]=useState<AnimationResponse|null>(null);
  const [action,setAction]=useState('attack_01');
  const [selectedTrack,setSelectedTrack]=useState('');
  const [newChannel,setNewChannel]=useState<NewTrackChannel>('bone-rotate');
  const [newTarget,setNewTarget]=useState('root');
  const [playing,setPlaying]=useState(false);
  const [playhead,setPlayhead]=useState(0);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');

  async function run(method:AnimationMethod,payload:Record<string,unknown>={}){
    if(!project)return;
    setBusy(true);setError('');
    try{
      const value=await invokeNative<AnimationResponse>(method,payload,project.revision);
      setProject(value.project);setData(value);
      const nextClip=value.animation.clips.find(item=>item.actionKey===action)??value.animation.clips[0];
      setAction(nextClip.actionKey);
      if(!nextClip.tracks.some(track=>track.trackId===selectedTrack))setSelectedTrack(nextClip.tracks[0]?.trackId??'');
      setPlayhead(current=>Math.min(current,nextClip.durationTicks));
    }catch(reason){setError(reason instanceof Error?reason.message:'Animation 命令失败')}
    finally{setBusy(false)}
  }

  useEffect(()=>{
    if(project?.gates.rig==='APPROVED'&&project.motionState==='PRESENT'&&project.animationState==='PRESENT')void run('animation.status');
  },[project?.projectId]);
  const clip=data?.animation.clips.find(item=>item.actionKey===action);
  useEffect(()=>{
    if(!playing||!clip)return;
    const timer=window.setInterval(()=>setPlayhead(value=>value>=clip.durationTicks?0:Math.min(clip.durationTicks,value+500)),16);
    return()=>window.clearInterval(timer);
  },[playing,clip?.durationTicks]);

  if(!project||project.gates.rig!=='APPROVED'||project.motionState!=='PRESENT')return <section className="animation-workspace"><div className="blocked-panel"><b>动画工作台已锁定</b><p>需要当前 Rig 审批和 MotionContent。</p></div></section>;
  if(project.animationState==='MISSING'&&!data)return <section className="animation-workspace"><header><div><span className="eyebrow">整数 tick 时间轴 · Spine 4.2.43</span><h1>动画工作台</h1><p>从已审批 Rig 与十动作 MotionSpec 创建可手工编辑的 Clip、姿势 marker 和三个独立 hit marker。</p></div><button className="primary" disabled={busy} onClick={()=>void run('animation.initialize')}>{busy?'正在建立…':'创建十动作时间轴候选'}</button></header>{error&&<div role="alert" className="error-banner">{error}</div>}</section>;
  if(!data||!clip)return <section className="animation-workspace"><div className="blocked-panel"><b>正在验证 AnimationSet</b><p>{error||'重算 Rig、Motion、Clip 与 marker 绑定…'}</p></div></section>;

  const row=data.actions.find(item=>item.definition.key===action)!;
  const markers=data.animation.reviewPoseMarkers.filter(item=>item.actionKey===action);
  const gameplay=data.animation.gameplayMarkers.filter(item=>item.actionKey===action);
  const issues=data.diagnostics.filter(issue=>issue.target===clip.clipId||clip.tracks.some(track=>track.targetId===issue.target));
  const track=clip.tracks.find(item=>item.trackId===selectedTrack)??clip.tracks[0];
  const allTicks=Array.from(new Set(clip.tracks.flatMap(item=>item.keyframes.map(key=>key.tick)))).sort((a,b)=>a-b);
  const approvedImages=data.motion.assets.filter(asset=>asset.actionKey===action&&asset.state==='approved').length;
  const requiredImages=data.motion.assets.filter(asset=>asset.actionKey===action).length;
  const contactPhase=data.motion.specs.find(spec=>spec.actionKey===action)?.phases.find(phase=>phase.key==='contact')??null;
  const primaryWeaponSockets=data.rig.sockets.filter(socket=>socket.kind==='primary-weapon');
  const targetOptions=(newChannel==='slot-color'||newChannel==='draw-order'
    ?data.rig.slotSet.slots.map(slot=>({id:slot.slotId,label:`${slot.slotId} · ${slot.layerId}`}))
    :data.rig.boneTree.bones.map(value=>({id:value.boneId,label:`${value.name} · ${value.boneId}`})));
  const resolvedTarget=targetOptions.some(value=>value.id===newTarget)?newTarget:(targetOptions[0]?.id??'');
  const duplicateTrack=clip.tracks.some(value=>value.targetId===resolvedTarget&&value.channel===newChannel);

  const selectAction=(key:string)=>{
    setAction(key);
    const next=data.animation.clips.find(item=>item.actionKey===key)!;
    setSelectedTrack(next.tracks[0]?.trackId??'');setPlayhead(0);setPlaying(false);
  };
  const updateKey=(keyframe:Keyframe)=>{
    const next={...track,keyframes:track.keyframes.map(key=>key.keyframeId===keyframe.keyframeId?keyframe:key)};
    void run('animation.track.put',{actionKey:action,track:next});
  };
  const addKey=()=>{
    const arity=track.channel==='bone-rotate'||track.channel==='draw-order'?1:track.channel==='slot-color'?4:2;
    const next={...track,keyframes:[...track.keyframes,{keyframeId:crypto.randomUUID(),tick:playhead,valuesMilli:Array(arity).fill(track.channel==='bone-scale'?1_000_000:0),curve:'linear' as const,bezierMilli:null}]};
    void run('animation.track.put',{actionKey:action,track:next});
  };
  const createTrack=()=>{
    if(!resolvedTarget||duplicateTrack)return;
    const values=newChannel==='bone-scale'?[1_000_000,1_000_000]:newChannel==='slot-color'?[1_000,1_000,1_000,1_000]:newChannel==='bone-rotate'||newChannel==='draw-order'?[0]:[0,0];
    const id=`track:${action}:${resolvedTarget}:${newChannel}:${crypto.randomUUID()}`;
    setSelectedTrack(id);
    void run('animation.track.put',{actionKey:action,track:{trackId:id,targetId:resolvedTarget,channel:newChannel,keyframes:[{keyframeId:`${id}:start`,tick:0,valuesMilli:values,curve:'linear',bezierMilli:null},{keyframeId:`${id}:end`,tick:clip.durationTicks,valuesMilli:values,curve:'linear',bezierMilli:null}]}});
  };

  const poseBlocked=busy||row.poseApproved||approvedImages!==requiredImages||issues.some(issue=>issue.severity==='P0'||issue.severity==='P1');
  return <section className="animation-workspace">
    <header><div><span className="eyebrow">{data.authority}</span><h1>{reviewMode?'动作审核台':'动画工作台'}</h1><p>Pixi 只消费当前 Rig、Clip 与 playhead；命中帧是独立 marker，不伪装为动画 track。</p></div><div><button className="primary" disabled={poseBlocked} title={approvedImages!==requiredImages?'需要先逐张批准该动作的关键姿势参考图':''} onClick={()=>void run('animation.pose.approve',{actionKey:action})}>{row.poseApproved?'Pose 已批准':'原生确认全部关键姿势'}</button>{row.definition.requiresHitFrame&&<button className="primary hit-approve" disabled={busy||!row.poseApproved||row.hitApproved} onClick={()=>void run('animation.hit.approve',{actionKey:action})}>{row.hitApproved?'Hit 已批准':'单独确认命中帧'}</button>}</div></header>
    {error&&<div role="alert" className="error-banner">{error}<button onClick={()=>setError('')}>关闭</button></div>}
    <div className="clip-strip" role="tablist">{data.actions.map(item=><button role="tab" aria-selected={action===item.definition.key} key={item.definition.key} className={action===item.definition.key?'active':''} onClick={()=>selectAction(item.definition.key)}><b>{item.definition.key}</b><span>POSE {item.poseApproved?'✓':'○'}{item.definition.requiresHitFrame?` · HIT ${item.hitApproved?'✓':'○'}`:''}</span></button>)}</div>
    <div className="animation-main">
      <aside className="track-tree"><header><b>{action}</b><em>clip revision {clip.revision}</em></header>{clip.tracks.map(item=><button key={item.trackId} className={track.trackId===item.trackId?'selected':''} onClick={()=>setSelectedTrack(item.trackId)}><i>◇</i><span>{item.targetId}</span><small>{item.channel}</small></button>)}<section><b>权威诊断</b>{issues.length?issues.map(issue=><p key={`${issue.code}-${issue.target}`}><i>○</i>{issue.severity} {issue.code}</p>):<p><i>✓</i>P0/P1 为 0</p>}<p><i>{approvedImages===requiredImages?'✓':'○'}</i>参考图 {approvedImages}/{requiredImages}</p></section></aside>
      <main className="preview-area"><PixiRigPreview rig={data.rig} clip={clip} playhead={playhead} attachmentPreview={data.attachmentPreview}/><div className="attachment-preview-boundary"><strong>{data.attachmentPreview.diagnostics.supportedAttachmentCount}/{data.attachmentPreview.attachments.length} 个附件参与动态预览</strong><span>仅刚性单骨全画布 sprite；slot color 与 draw order 会采样。Mesh/deform 和多骨蒙皮未模拟，也不会伪装为已验证。</span>{data.attachmentPreview.diagnostics.unsupportedLayerIds.length>0&&<code>未支持：{data.attachmentPreview.diagnostics.unsupportedLayerIds.join(', ')}</code>}</div><div className="preview-controls"><button aria-label={playing?'暂停':'播放'} onClick={()=>setPlaying(value=>!value)}>{playing?'❚❚':'▶'}</button><button aria-label="回到起点" onClick={()=>{setPlaying(false);setPlayhead(0)}}>■</button><span>{playhead} / {clip.durationTicks} tick</span></div></main>
      <aside className="anim-inspector">
        <section><b>审核姿势 marker</b><ul className="pose-marker-list">{markers.map(marker=><li key={marker.markerId}><label>{marker.poseKey}<select value={marker.tick} disabled={busy} onChange={event=>void run('animation.poseMarker.set',{actionKey:action,poseKey:marker.poseKey,tick:Number(event.target.value)})}>{allTicks.map(tick=><option key={tick} value={tick}>{tick}</option>)}</select></label></li>)}</ul></section>
        {row.definition.requiresHitFrame&&<section><b>独立 Gameplay marker</b>{gameplay.map(marker=><HitMarkerEditor key={marker.markerId} marker={marker} contactPhase={contactPhase} sockets={primaryWeaponSockets} playhead={playhead} busy={busy} onSave={(tick,socketId)=>void run('animation.hitMarker.set',{expectedAnimationRevision:data.animation.revision,actionKey:action,tick,socketId})}/>)}</section>}
        <section className="new-track-form"><b>新增可编辑 Track</b><label>通道<select value={newChannel} disabled={busy} onChange={event=>setNewChannel(event.target.value as NewTrackChannel)}><option value="bone-translate">Bone Translate</option><option value="bone-rotate">Bone Rotate</option><option value="bone-scale">Bone Scale</option><option value="slot-color">Slot Color</option><option value="draw-order">Draw Order</option></select></label><label>目标<select value={resolvedTarget} disabled={busy} onChange={event=>setNewTarget(event.target.value)}>{targetOptions.map(value=><option key={value.id} value={value.id}>{value.label}</option>)}</select></label><button className="secondary" disabled={busy||!resolvedTarget||duplicateTrack} onClick={createTrack}>{duplicateTrack?'该目标/通道已存在':'创建首尾关键帧 Track'}</button></section>
        <section><b>选中 Track</b><p>{track.targetId} · {track.channel}</p><button className="secondary" disabled={busy||track.keyframes.some(key=>key.tick===playhead)} onClick={addKey}>在当前 tick 添加关键帧</button><div className="keyframe-editor">{track.keyframes.map(key=><KeyframeCard key={key.keyframeId} value={key} busy={busy} onCommit={updateKey}/>)}</div></section>
        <section className="game-boundary"><b>玩法边界</b><p>marker 不包含伤害、碰撞箱、无敌判定或连招逻辑；这些属于目标游戏运行时。</p></section>
      </aside>
    </div>
    <RealTimeline clip={clip} markers={markers} gameplay={gameplay} playhead={playhead} setPlayhead={setPlayhead}/>
  </section>;
}

function HitMarkerEditor({marker,contactPhase,sockets,playhead,busy,onSave}:{marker:GameplayMarker;contactPhase:{startTick:number;endTick:number}|null;sockets:Array<{socketId:string;semantic:string}>;playhead:number;busy:boolean;onSave:(tick:number,socketId:string)=>void}){
  const[tickDraft,setTickDraft]=useState(String(marker.startTick));const[socketDraft,setSocketDraft]=useState(marker.socketId??sockets[0]?.socketId??'');
  useEffect(()=>{setTickDraft(String(marker.startTick));setSocketDraft(marker.socketId??sockets[0]?.socketId??'')},[marker.startTick,marker.socketId,sockets.map(socket=>socket.socketId).join('|')]);
  const tick=Number(tickDraft);const valid=Boolean(contactPhase)&&Number.isInteger(tick)&&tick>=contactPhase!.startTick&&tick<=contactPhase!.endTick&&sockets.some(socket=>socket.socketId===socketDraft);const dirty=valid&&(tick!==marker.startTick||socketDraft!==marker.socketId);
  return <div className="hit-marker-editor"><dl><div><dt>类型</dt><dd>{marker.kind}</dd></div><div><dt>允许区间</dt><dd>{contactPhase?`${contactPhase.startTick}–${contactPhase.endTick}`:'MotionSpec 缺 contact phase'}</dd></div></dl><label>命中 tick<input type="number" min={contactPhase?.startTick} max={contactPhase?.endTick} step={1} value={tickDraft} aria-invalid={!valid} disabled={busy||!contactPhase} onChange={event=>setTickDraft(event.target.value)}/></label><button className="secondary" disabled={busy||!contactPhase||playhead<contactPhase.startTick||playhead>contactPhase.endTick} onClick={()=>setTickDraft(String(playhead))}>使用当前播放头</button><label>主武器 socket<select value={socketDraft} disabled={busy||sockets.length===0} onChange={event=>setSocketDraft(event.target.value)}>{sockets.map(socket=><option key={socket.socketId} value={socket.socketId}>{socket.semantic} · {socket.socketId}</option>)}</select></label><button className="primary" disabled={busy||!dirty} onClick={()=>onSave(tick,socketDraft)}>保存命中帧</button><small>保存会使当前 Hit 审批失效，但不会撤销已完成的 Pose 审批。</small></div>;
}

function KeyframeCard({value,busy,onCommit}:{value:Keyframe;busy:boolean;onCommit:(value:Keyframe)=>void}){
  const[tick,setTick]=useState(String(value.tick));const[values,setValues]=useState(value.valuesMilli.join(','));const[curve,setCurve]=useState<Keyframe['curve']>(value.curve);
  useEffect(()=>{setTick(String(value.tick));setValues(value.valuesMilli.join(','));setCurve(value.curve)},[value.tick,value.valuesMilli.join(','),value.curve]);
  const parsedValues=values.split(',').map(item=>Number(item.trim()));const parsedTick=Number(tick);const valid=Number.isInteger(parsedTick)&&parsedTick>=0&&parsedValues.length>0&&parsedValues.every(Number.isFinite);const dirty=valid&&(parsedTick!==value.tick||values!==value.valuesMilli.join(',')||curve!==value.curve);
  return <article><label>Tick<input type="number" value={tick} disabled={busy} onChange={event=>setTick(event.target.value)}/></label><label>Values<input value={values} disabled={busy} aria-invalid={!valid} onChange={event=>setValues(event.target.value)}/></label><label>Curve<select value={curve} disabled={busy} onChange={event=>setCurve(event.target.value as Keyframe['curve'])}><option value="linear">Linear</option><option value="stepped">Stepped</option><option value="bezier">Bezier</option></select></label><button className="secondary" disabled={busy||!dirty} onClick={()=>onCommit({...value,tick:parsedTick,valuesMilli:parsedValues,curve,bezierMilli:curve==='bezier'?(value.bezierMilli??[250,250,750,750]):null})}>保存此关键帧</button></article>;
}

function RealTimeline({clip,markers,gameplay,playhead,setPlayhead}:{clip:Clip;markers:PoseMarker[];gameplay:GameplayMarker[];playhead:number;setPlayhead:(tick:number)=>void}){
  const pct=(tick:number)=>`${tick/clip.durationTicks*100}%`;
  return <section className="timeline"><header><div className="timeline-spacer">Tracks / Markers</div><div className="ruler">{Array.from({length:7},(_,index)=>Math.round(clip.durationTicks*index/6)).map(tick=><button key={tick} onClick={()=>setPlayhead(tick)}>{tick}</button>)}<i style={{left:pct(playhead)}}/></div></header>{clip.tracks.map((track,row)=><div className="timeline-row" key={track.trackId}><label>{track.targetId}<small>{track.channel}</small></label><div>{track.keyframes.map(key=><button aria-label={`${track.targetId} at ${key.tick}`} key={key.keyframeId} onClick={()=>setPlayhead(key.tick)} style={{left:pct(key.tick)}}/>)}{row===0&&<i className="playhead" style={{left:pct(playhead)}}/>}</div></div>)}<div className="timeline-row marker-row"><label>Review poses<small>非 gameplay</small></label><div>{markers.map(marker=><button aria-label={`${marker.poseKey} at ${marker.tick}`} title={marker.poseKey} key={marker.markerId} onClick={()=>setPlayhead(marker.tick)} style={{left:pct(marker.tick)}}/>)}</div></div>{gameplay.length>0&&<div className="timeline-row marker-row"><label>Gameplay<small>hit marker</small></label><div>{gameplay.map(marker=><button className="hit" aria-label={`${marker.kind} at ${marker.startTick}`} key={marker.markerId} onClick={()=>setPlayhead(marker.startTick)} style={{left:pct(marker.startTick)}}/>)}</div></div>}</section>;
}
