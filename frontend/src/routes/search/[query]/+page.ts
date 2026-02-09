import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params }) => {
	const { query } = params;

	// TODO: Load search content here.

	return {
		query
	};
};
