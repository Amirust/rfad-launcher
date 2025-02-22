export enum UpdateStatus {
  DownloadStarted = 0,
  DownloadFinished,
  UnpackStarted,
  UnpackFinished,
}

export interface UpdateProgress {
  status: UpdateStatus;
}

export interface DownloadProgress {
  fileName: string;
  downloadBytes: number;
  percentage: number;
  speedBytesPerSec: number;
}

export enum EventNames {
  UpdateProgress = 'update:progress',
  DownloadProgress = 'download:progress',
}