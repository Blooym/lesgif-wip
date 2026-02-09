export enum UploadStep {
	SelectFile = 'select-file',
	ContentDetails = 'content-details',
	UploadProgress = 'upload-progress',
	Complete = 'complete'
}

export interface ContentLabels {
	nsfw: boolean;
	photosensitive: boolean;
	aiGeneration: boolean;
}

export interface UploadState {
	file: File | null;
	previewUrl: string | null;
	title: string;
	tags: string[];
	labels: ContentLabels;
}

export interface ProgressState {
	status: 'idle' | 'uploading' | 'processing' | 'complete' | 'error';
	message: string;
}
