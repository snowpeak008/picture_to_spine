export type CliEvidenceState = 'NOT_RUN' | 'FAILED' | 'SUCCEEDED';
export type CliJobState = 'QUEUED' | 'AWAITING_NATIVE_INPUT' | 'PREPARING_CONSENT' | 'AWAITING_HUMAN_CONFIRMATION' | 'RUNNING' | 'NOT_RUN' | 'FAILED' | 'SUCCEEDED';

export interface SpineCliStatus {
  schemaVersion: '1.0.0';
  configured: boolean;
  assessment: {
    evidenceClass: 'EXTERNAL'; state: CliEvidenceState; reasonCode: string;
    pathToken: string | null; executableSha256: string | null; expectedPatch: '4.2.43';
    professionalLicenseConfirmed: boolean; confirmedAtUnixMs: number | null;
    observedPatch: string | null; realCliTested: boolean;
  };
  openExports: Array<{ exportId: string; projectId: string; projectRevision: number; snapshotSha256: string }>;
  policy: { bundled: false; downloadedByApp: false; activationDataRead: false; networkGranted: false; absolutePathReturnedToWebView: false };
}

export interface SpineCliJob {
  schemaVersion: 'f2s-spine-cli-job/1.0';
  jobId: string; operationId: string;
  operationKind: 'IMPORT_PROJECT' | 'PACK_ATLAS' | 'EXPORT_BINARY';
  exportId: string; evidenceClass: 'EXTERNAL'; state: CliJobState; failureCode: string | null;
  outputPathToken: string | null;
  outputs: Array<{ relativePath: string; extension: string; sha256: string; authorized: boolean }>;
  provenanceSha256: string | null; createdAtUnixMs: number; finishedAtUnixMs: number | null;
}

export function cliJobTerminal(state: CliJobState) {
  return state === 'SUCCEEDED' || state === 'FAILED' || state === 'NOT_RUN';
}
