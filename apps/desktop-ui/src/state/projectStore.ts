import { create } from 'zustand';

export interface PrimaryWeaponProjection {
  weaponType: string;
  gripMode: 'one-hand' | 'two-hand' | 'flexible';
  weaponHand: 'near-hand' | 'far-hand' | 'both-hands';
  socketSemantic: string;
  sizeClass: 'small' | 'medium' | 'large';
  silhouetteConstraints: string;
}

export interface StyleSpecProjection {
  revision: number;
  viewpoint: 'side-view';
  renderingStyle: string;
  outline: string;
  paletteNotes: string;
  identityNotes: string;
  primaryWeapon: PrimaryWeaponProjection | null;
}

export interface MasterProjection {
  masterId: string;
  sourceArtifactId: string;
  candidateRevision: number;
  sourceSha256: string;
  styleSpec: StyleSpecProjection;
  approvalState: string;
  supersedes: string | null;
}

export interface LayerProjection {
  layerId: string;
  name: string;
  role: string;
  attachmentSha256: string;
  maskSha256: string;
  visible: boolean;
  approved: boolean;
}

export interface LayerSetProjection {
  layerSetId: string;
  masterId: string;
  revision: number;
  layers: LayerProjection[];
  approvalState: string;
}

export interface ProjectProjection {
  availability: 'AVAILABLE' | 'INTEGRITY_CHECK_FAILED';
  diagnosticCode: string | null;
  projectId: string;
  displayName: string;
  revision: number;
  workflowStage: string;
  sourceCount: number;
  masterState: string;
  layerState: string;
  rigState: 'PENDING' | 'APPROVED' | null;
  motionState: 'PRESENT' | 'MISSING';
  animationState: 'PRESENT' | 'MISSING';
  poseApprovalCount: number;
  hitApprovalCount: number;
  activeMaster: MasterProjection | null;
  activeLayerSet: LayerSetProjection | null;
  gates: { master: string; layers: string; rig: string; poses: string; hits: string };
}

interface ProjectState {
  project: ProjectProjection | null;
  setProject: (project: ProjectProjection | null) => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  project: null,
  setProject: (project) => set({ project }),
}));
