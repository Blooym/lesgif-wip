<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	interface Props extends HTMLButtonAttributes {
		variant?: 'neutral' | 'destructive' | 'primary';
		size?: 'small' | 'normal';
		surface?: 'base' | 'mantle';
		class?: string;
		children: Snippet;
	}

	let {
		variant = 'primary',
		size = 'normal',
		surface = 'base',
		class: className,
		children,
		...restProps
	}: Props = $props();
</script>

<button class="button variant-{variant} size-{size} surface-{surface} {className}" {...restProps}>
	{@render children()}
</button>

<style>
	.button {
		border: none;
		border-radius: var(--radius-sm);
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
		flex-shrink: 0;

		&:disabled {
			cursor: not-allowed;
			opacity: 0.5;
		}
	}

	/* Sizes */
	.button.size-small {
		padding: 6px 12px;
		font-size: 0.7rem;
	}

	.button.size-normal {
		padding: 8px 16px;
		font-size: 0.8rem;
	}

	/* Variants */
	.button.variant-primary {
		background: var(--ctp-mauve);
		color: var(--ctp-crust);

		&:hover:not(:disabled) {
			background: var(--ctp-lavender);
		}
	}

	.button.variant-destructive {
		background: var(--ctp-red);
		color: var(--ctp-crust);

		&:hover:not(:disabled) {
			background: var(--ctp-maroon);
		}
	}

	.button.variant-neutral {
		color: var(--ctp-text);

		&:hover:not(:disabled) {
			border-color: var(--ctp-mauve);
			color: var(--ctp-mauve);
		}
	}

	/* Surface variants for neutral */
	.button.variant-neutral.surface-base {
		background: transparent;
		border: var(--border-md) solid var(--ctp-surface0);
	}

	.button.variant-neutral.surface-mantle {
		background: var(--ctp-crust);
		border: var(--border-md) solid var(--ctp-surface1);
	}
</style>
