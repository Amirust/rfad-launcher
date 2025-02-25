export enum UpdateStatus {
  DownloadStarted = 0,
  DownloadFinished,
  UnpackStarted,
  UnpackFinished,
  LoadOrderUpdateStarted,
  LoadOrderUpdateFinished,
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

export interface UnpackProgress {
  percentage: number;
}

export enum EventNames {
  UpdateProgress = 'update:progress',
  DownloadProgress = 'download:progress',
  UnpackProgress = 'unpack:progress',
}