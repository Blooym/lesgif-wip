<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	interface Props extends Omit<HTMLInputAttributes, 'size' | 'value'> {
		surface?: 'base' | 'mantle';
		size?: 'small' | 'normal';
		value?: string;
		class?: string;
	}

	let {
		surface = 'base',
		size = 'normal',
		value = $bindable(''),
		class: className = '',
		...restProps
	}: Props = $props();
</script>

<input class="{className} surface-{surface} size-{size}" bind:value {...restProps} />

<style>
	/* Surfaces */
	input.surface-base {
		background: var(--ctp-mantle);
		border: var(--border-md) solid var(--ctp-surface0);
	}

	input.surface-mantle {
		background: var(--ctp-crust);
		border: var(--border-md) solid var(--ctp-surface1);
	}

	/* Sizes */
	input.size-small {
		padding: 6px 12px;
		font-size: 0.825rem;
	}

	input.size-normal {
		padding: 8px 16px;
		font-size: 0.9rem;
	}

	/* Component */
	input {
		width: 100%;
		border-radius: var(--radius-md);
		color: var(--ctp-text);
		font-family: inherit;
		transition: border-color 0.2s;

		&:focus {
			outline: none;
			border-color: var(--ctp-mauve);
		}
		&::placeholder {
			color: var(--ctp-subtext0);
		}
	}
</style>
