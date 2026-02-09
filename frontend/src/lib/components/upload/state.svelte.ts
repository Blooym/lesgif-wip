import { UploadStep, type ContentLabels, type ProgressState, type UploadState } from './types';

export const MAX_TAGS = 5;
export const MAX_FILE_SIZE_MB = 10;
export const MAX_FILE_SIZE_BYTES = MAX_FILE_SIZE_MB * 1024 * 1024;
export const ACCEPTED_TYPES = ['GIF', 'WebP'];

function createUploadState() {
	let currentStep = $state<UploadStep>(UploadStep.SelectFile);
	let error = $state('');

	let upload = $state<UploadState>({
		file: null,
		previewUrl: null,
		title: '',
		tags: [],
		labels: { nsfw: false, photosensitive: false, aiGeneration: false }
	});

	let progress = $state<ProgressState>({
		status: 'idle',
		message: ''
	});

	function setFile(file: File | null, previewUrl: string | null) {
		upload.file = file;
		upload.previewUrl = previewUrl;
	}

	function clearFile() {
		upload.file = null;
		upload.previewUrl = null;
	}

	function setTitle(title: string) {
		upload.title = title;
	}

	function addTag(tag: string) {
		const trimmed = tag.trim();
		if (trimmed && upload.tags.length < MAX_TAGS && !upload.tags.includes(trimmed)) {
			upload.tags = [...upload.tags, trimmed];
			return true;
		}
		return false;
	}

	function removeTag(index: number) {
		upload.tags = upload.tags.filter((_, i) => i !== index);
	}

	function setLabel<K extends keyof ContentLabels>(key: K, value: boolean) {
		upload.labels[key] = value;
	}

	function setStep(step: UploadStep) {
		error = '';
		currentStep = step;
	}

	function setError(msg: string) {
		error = msg;
	}

	function setProgress(status: ProgressState['status'], message: string) {
		progress.status = status;
		progress.message = message;
	}

	function reset() {
		currentStep = UploadStep.SelectFile;
		error = '';
		upload = {
			file: null,
			previewUrl: null,
			title: '',
			tags: [],
			labels: { nsfw: false, photosensitive: false, aiGeneration: false }
		};
		progress = { status: 'idle', message: '' };
	}

	return {
		get currentStep() {
			return currentStep;
		},
		get error() {
			return error;
		},
		get upload() {
			return upload;
		},
		get progress() {
			return progress;
		},

		setFile,
		clearFile,
		setTitle,
		addTag,
		removeTag,
		setLabel,
		setStep,
		setError,
		setProgress,
		reset
	};
}

export const uploadState = createUploadState();
