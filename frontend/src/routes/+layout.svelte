<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import AuthenticationDialog from '$lib/components/auth/AuthenticationDialog.svelte';
	import SiteFooter from '$lib/components/site/SiteFooter.svelte';
	import SiteHeader from '$lib/components/site/SiteHeader.svelte';
	import '$lib/styles/behaviour.css';
	import '$lib/styles/reset.css';
	import '$lib/styles/theme.css';
	import '@fontsource-variable/inter';
	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>Gifdex</title>
</svelte:head>

<div class="layout">
	<SiteHeader />

	<main>
		{@render children()}
	</main>

	<SiteFooter />
</div>

<AuthenticationDialog />

<style>
	.layout {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	main {
		flex: 1;
		max-width: 1400px;
		width: 100%;
		margin: 1.5rem auto;
		padding: 0 1rem;
	}
</style>
