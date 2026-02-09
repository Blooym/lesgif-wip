<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import AccountSwitcher from '$lib/components/auth/AccountDropdown.svelte';
	import Button from '$lib/components/base/button/Button.svelte';
	import { authStore } from '$lib/stores/auth.svelte';
	import Input from '../base/input/Input.svelte';

	let searchQuery = $state<string>('');
</script>

<header>
	<div class="header-content">
		<div class="header-top">
			<a href={'/'} class="logo">Gifdex</a>
			<nav class="actions">
				{#if authStore.isAuthenticated()}
					<Button onclick={() => {}} variant="primary">Upload</Button>
					<AccountSwitcher />
				{:else}
					<Button
						variant="neutral"
						surface="mantle"
						onclick={() => {
							authStore.showSignInDialog = true;
						}}>Sign in</Button
					>
				{/if}
			</nav>
		</div>
		<div class="header-search">
			<form
				onsubmit={(event) => {
					event.preventDefault();
					goto(
						resolve(`/search/[query]`, {
							query: searchQuery
						})
					);
				}}
			>
				<Input
					type="search"
					size="normal"
					surface="mantle"
					placeholder="Search for GIFs or profiles..."
					bind:value={searchQuery}
				/>
			</form>
		</div>
	</div>
</header>

<style>
	header {
		/* Always stick to top of page */
		position: sticky;
		z-index: 100;
		top: 0;

		background: var(--ctp-mantle);
		border-bottom: 1px solid var(--ctp-surface0);
		padding: 12px 16px;
		width: 100%;

		.header-content {
			max-width: 1300px;
			margin: 0 auto;
			display: flex;
			flex-direction: column;
			align-items: center;
			row-gap: 6px;

			.header-top {
				display: flex;
				width: 100%;
				justify-content: space-between;

				.logo {
					text-decoration: none;
					font-size: 1.5rem;
					font-weight: 700;
					color: var(--ctp-text);
				}

				.actions {
					display: flex;
					align-items: center;
					gap: 10px;
				}
			}

			.header-search {
				width: 100%;
				position: relative;
			}
		}
	}
</style>
