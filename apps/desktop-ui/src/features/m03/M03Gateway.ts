export interface PreflightResult { mediaType:string;width:number;height:number;bitDepth:number;byteLength:number;capabilityState:'VERIFIED'|'UNVERIFIED';diagnostics:string[] }
export interface M03Gateway { chooseAndPreflightImage():Promise<PreflightResult>;createMaster(style:StyleSpecDraft):Promise<{masterId:string;revision:number}>;approveMaster(masterId:string,revision:number):Promise<void> }
export interface PrimaryWeaponDraft { weaponType:string;gripMode:'one-hand'|'two-hand'|'flexible';weaponHand:'near-hand'|'far-hand'|'both-hands';socketSemantic:string;sizeClass:'small'|'medium'|'large';silhouetteConstraints:string }
export interface StyleSpecDraft { viewpoint:'side-view';renderingStyle:string;outline:string;paletteNotes:string;identityNotes:string;primaryWeapon:PrimaryWeaponDraft|null }
export function isNativeGatewayAvailable(){return typeof window!=='undefined'&&'chrome'in window&&typeof (window as unknown as {chrome?:{webview?:unknown}}).chrome?.webview==='object';}
