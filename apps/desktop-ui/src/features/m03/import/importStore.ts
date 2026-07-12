import { create } from 'zustand';
export interface ImportCandidate{fileName:string;mediaType:string;byteLength:number;width:number;height:number;bitDepth:number;sourceSha256:string;stagingToken:string;completeDecode:boolean;status:'RUST_PREFLIGHT'|'CAS_CANDIDATE'|'REJECTED'}
interface ImportState{candidate:ImportCandidate|null;error:string|null;setCandidate:(candidate:ImportCandidate)=>void;setError:(error:string)=>void;clear:()=>void}
export const useImportStore=create<ImportState>(set=>({candidate:null,error:null,setCandidate:candidate=>set({candidate,error:null}),setError:error=>set({candidate:null,error}),clear:()=>set({candidate:null,error:null})}));
