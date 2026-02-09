<script>
	import { authStore } from '$lib/stores/auth.svelte';
	import Button from '../base/button/Button.svelte';
	import Dialog from '../base/dialog/Dialog.svelte';
	import Input from '../base/input/Input.svelte';
	import Link from '../base/link/Link.svelte';

	let identity = $state('');
</script>

<Dialog bind:open={authStore.showSignInDialog}>
	<div class="dialog-content">
		<div class="header">
			<h2>Sign in to Gifdex</h2>
			<p>
				Sign in with your <span class="identity-phrase">Atmosphere Account</span> to get started
			</p>
		</div>
		<div class="form-section">
			<form
				onsubmit={(e) => {
					e.preventDefault();
					authStore.initiateOAuthFlow(identity);
				}}
			>
				<div class="input-group">
					<label for="identity-input">Handle or domain</label>
					<Input
						id="identity-input"
						required
						surface="mantle"
						bind:value={identity}
						placeholder="jane-doe.example.com"
					/>
				</div>
				<Button type="submit" variant="primary" size="normal">Sign In</Button>
			</form>
			<span class="divider">or</span>
			<!-- TODO: Add multiple providers to pick from. -->
			<Button
				variant="neutral"
				surface="mantle"
				size="normal"
				onclick={() => authStore.initiateOAuthFlow('https://bsky.social')}
				>Sign up via Bluesky</Button
			>
		</div>
		<small class="terms">
			By signing in you agree to our <Link target="_blank" href="/legal/privacy"
				>privacy policy</Link
			>
			and
			<Link target="_blank" href="/legal/terms">terms of service</Link>
		</small>
	</div>
</Dialog>

<style>
	.dialog-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 35px;
	}

	.header {
		text-align: center;

		h2 {
			margin: 0;
		}

		p {
			margin: 0;
			color: var(--ctp-subtext0);

			.identity-phrase {
				color: var(--ctp-mauve);
			}
		}
	}

	.form-section {
		display: flex;
		flex-direction: column;
		width: min(100%, 280px);
		gap: 0.75rem;
	}

	.form-section form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.input-group {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.input-group label {
		font-size: 0.8125rem;
		color: var(--ctp-subtext1);
		padding-left: 0.125rem;
	}

	.divider {
		display: flex;
		align-items: center;
		gap: 0.7rem;
		font-size: 0.75rem;
		color: var(--ctp-subtext0);
		&::before,
		&::after {
			content: '';
			flex: 1;
			height: 1px;
			background-color: var(--ctp-surface2);
		}
	}

	.terms {
		text-align: center;
		color: var(--ctp-subtext0);
	}
</style>
