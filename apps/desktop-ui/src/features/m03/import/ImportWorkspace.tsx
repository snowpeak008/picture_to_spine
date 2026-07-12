import { useState } from 'react';
import { invokeNative, nativeAvailable } from '../../../native/ipc';
import { useProjectStore, type ProjectProjection } from '../../../state/projectStore';
import { MasterReview } from '../master/MasterReview';
import { useImportStore } from './importStore';
import { PreflightPanel } from './PreflightPanel';
import './native-import.css';

interface ChooseResult { cancelled:boolean; stagingToken?:string; fileName?:string; report?:{mediaType:string;width:number;height:number;bitDepth:number;sourceSha256:string;encodedBytes:number;completeDecode:boolean} }
interface PromoteResult { project:ProjectProjection }

export function ImportWorkspace(){
  const {candidate,error,setCandidate,setError,clear}=useImportStore();
  const {project,setProject}=useProjectStore();
  const [promoting,setPromoting]=useState(false);const native=nativeAvailable();
  async function choose(){if(!project)return;try{const value=await invokeNative<ChooseResult>('image.chooseAndPreflight',{},project.revision);if(value.cancelled)return;const report=value.report;if(!report||!value.stagingToken)throw new Error('Native preflight response incomplete');setCandidate({fileName:value.fileName??'selected-image',mediaType:report.mediaType,byteLength:report.encodedBytes,width:report.width,height:report.height,bitDepth:report.bitDepth,sourceSha256:report.sourceSha256,stagingToken:value.stagingToken,completeDecode:report.completeDecode,status:'RUST_PREFLIGHT'});}catch(e){setError(e instanceof Error?e.message:'图片预检失败')}}
  async function promote(){if(!candidate||!project)return;setPromoting(true);try{const value=await invokeNative<PromoteResult>('image.promote',{stagingToken:candidate.stagingToken},project.revision);setProject(value.project);setCandidate({...candidate,status:'CAS_CANDIDATE'});}catch(e){setError(e instanceof Error?e.message:'CAS 提升失败')}finally{setPromoting(false)}}
  const hasPersistedSource=Boolean(project&&project.sourceCount>0);
  return <section className="m03-workspace"><header><div><span className="eyebrow">人工门 01 · Native revision chain</span><h1>输入与母版</h1><p>先确认角色风格、身份和主武器语义，再允许分层、Rig 和动作继续。</p></div><button className="primary" onClick={()=>void choose()} disabled={!native}>通过 Windows 对话框选择图片</button></header>{!native&&<div className="error-banner"><b>PROTOTYPE</b>浏览器预览不具备 Native IPC；为避免不受限解码，图片选择已禁用。请运行桌面 EXE。</div>}{error&&<div className="error-banner"><b>F2S-IMPORT-001</b>{error}<button onClick={clear}>关闭</button></div>}<div className="import-grid"><div className="image-stage">{candidate?<div className="native-preview-withheld"><span>✓</span><strong>{candidate.fileName}</strong><small>Native 完整解码已通过；原图不传给浏览器。分层阶段只使用 Native 生成的受限预览。</small></div>:hasPersistedSource?<div className="native-preview-withheld"><span>✓</span><strong>已从本地项目头恢复图片引用</strong><small>原图留在 CAS；如重新选择图片，会使母版及下游审批失效。</small></div>:<div className="empty-preview"><span>＋</span><strong>选择一张横版侧视角色图</strong><small>图片不会自动上传，也不会先交给浏览器解码</small></div>}</div>{candidate?<PreflightPanel candidate={candidate}/>:<aside className="import-guidance"><b>输入要求</b><ul><li>二次元类人、完整可见轮廓</li><li>严格横版侧视</li><li>一件主武器、无遮挡关键关节</li><li>PNG/JPEG/WebP，8-bit</li></ul></aside>}</div>{candidate&&candidate.status==='RUST_PREFLIGHT'&&<div className="candidate-actions"><button className="secondary" onClick={clear}>移除候选</button><button className="primary" disabled={promoting} onClick={()=>void promote()}>{promoting?'正在写入 CAS…':'提升为 CAS 候选'}</button><span>提升后仍需 StyleSpec 与人类母版审批。</span></div>}{candidate?.status==='CAS_CANDIDATE'&&<div className="candidate-actions"><span className="good">CAS 候选已绑定：{candidate.sourceSha256.slice(0,12)}…</span></div>}{hasPersistedSource&&<MasterReview/>}</section>;
}
