<script lang="ts">
	import { auth } from '$lib/stores/auth';
	import { goto } from '$app/navigation';

	let token = $state('');
	let error = $state('');
	let isLoading = $state(false);

	async function handleLogin() {
		if (!token.trim()) {
			error = 'Please enter a token';
			return;
		}

		isLoading = true;
		error = '';

		try {
			auth.login(token);
			const isValid = await auth.checkAuth();

			if (isValid) {
				goto('/');
			} else {
				error = 'Invalid token';
				token = '';
			}
		} catch (err) {
			error = 'Authentication failed. Please check your token.';
			token = '';
		} finally {
			isLoading = false;
		}
	}
</script>

<div class="login-container">
	<div class="login-card">
		<h1>Dashboard Login</h1>
		<p class="subtitle">Enter your dashboard token to continue</p>

		<form onsubmit={(e) => { e.preventDefault(); handleLogin(); }}>
			<div class="form-group">
				<label for="token">Token</label>
				<input
					id="token"
					type="password"
					bind:value={token}
					placeholder="Enter your token here"
					disabled={isLoading}
					autocomplete="off"
				/>
			</div>

			{#if error}
				<div class="error">{error}</div>
			{/if}

			<button type="submit" disabled={isLoading}>
				{isLoading ? 'Authenticating...' : 'Login'}
			</button>
		</form>

		<div class="help-text">
			<p>Don't have a token?</p>
			<p>Use the <code>/dashboard create-token</code> command in Discord to create one.</p>
		</div>
	</div>
</div>

<style>
	.login-container {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
		padding: 1rem;
	}

	.login-card {
		background: white;
		border-radius: 8px;
		padding: 2rem;
		max-width: 400px;
		width: 100%;
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
	}

	h1 {
		margin: 0 0 0.5rem 0;
		font-size: 1.75rem;
		color: #333;
	}

	.subtitle {
		margin: 0 0 2rem 0;
		color: #666;
		font-size: 0.9rem;
	}

	.form-group {
		margin-bottom: 1.5rem;
	}

	label {
		display: block;
		margin-bottom: 0.5rem;
		color: #333;
		font-weight: 500;
	}

	input {
		width: 100%;
		padding: 0.75rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 1rem;
		font-family: monospace;
		box-sizing: border-box;
	}

	input:focus {
		outline: none;
		border-color: #667eea;
		box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
	}

	input:disabled {
		background: #f5f5f5;
		cursor: not-allowed;
	}

	button {
		width: 100%;
		padding: 0.75rem;
		background: #667eea;
		color: white;
		border: none;
		border-radius: 4px;
		font-size: 1rem;
		font-weight: 500;
		cursor: pointer;
		transition: background 0.2s;
	}

	button:hover:not(:disabled) {
		background: #5568d3;
	}

	button:disabled {
		background: #ccc;
		cursor: not-allowed;
	}

	.error {
		background: #fee;
		color: #c33;
		padding: 0.75rem;
		border-radius: 4px;
		margin-bottom: 1rem;
		font-size: 0.9rem;
	}

	.help-text {
		margin-top: 2rem;
		padding-top: 1.5rem;
		border-top: 1px solid #eee;
		color: #666;
		font-size: 0.85rem;
	}

	.help-text p {
		margin: 0.5rem 0;
	}

	code {
		background: #f5f5f5;
		padding: 0.2rem 0.4rem;
		border-radius: 3px;
		font-family: monospace;
		color: #667eea;
	}
</style>
