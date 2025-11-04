<script lang="ts">
	import { auth } from '$lib/stores/auth';

	function handleLogout() {
		auth.logout();
	}
</script>

<div class="dashboard">
	<header>
		<h1>Ayaya Dashboard</h1>
		<div class="user-info">
			<span>User ID: {$auth.userId}</span>
			<button onclick={handleLogout} class="logout-btn">Logout</button>
		</div>
	</header>

	<main>
		<div class="welcome-card">
			<h2>Welcome to the Dashboard</h2>
			<p>You are successfully authenticated.</p>
		</div>

		<div class="info-section">
			<h3>Getting Started</h3>
			<ul>
				<li>Your dashboard token is securely stored in this browser</li>
				<li>Use Discord commands to manage your tokens</li>
				<li>
					<code>/dashboard list-tokens</code> - View your active tokens
				</li>
				<li>
					<code>/dashboard revoke-my-token</code> - Revoke a token
				</li>
			</ul>
		</div>
	</main>
</div>

<style>
	.dashboard {
		min-height: 100vh;
		background: #f5f5f5;
	}

	header {
		background: white;
		padding: 1.5rem 2rem;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	header h1 {
		margin: 0;
		font-size: 1.5rem;
		color: #333;
	}

	.user-info {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.user-info span {
		color: #666;
		font-size: 0.9rem;
		font-family: monospace;
	}

	.logout-btn {
		padding: 0.5rem 1rem;
		background: #dc3545;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.9rem;
		transition: background 0.2s;
	}

	.logout-btn:hover {
		background: #c82333;
	}

	main {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem;
	}

	.welcome-card {
		background: white;
		padding: 2rem;
		border-radius: 8px;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		margin-bottom: 2rem;
	}

	.welcome-card h2 {
		margin: 0 0 0.5rem 0;
		color: #333;
	}

	.welcome-card p {
		margin: 0;
		color: #666;
	}

	.info-section {
		background: white;
		padding: 2rem;
		border-radius: 8px;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.info-section h3 {
		margin: 0 0 1rem 0;
		color: #333;
	}

	.info-section ul {
		margin: 0;
		padding-left: 1.5rem;
		color: #666;
	}

	.info-section li {
		margin-bottom: 0.75rem;
	}

	code {
		background: #f5f5f5;
		padding: 0.2rem 0.4rem;
		border-radius: 3px;
		font-family: monospace;
		color: #667eea;
		font-size: 0.9em;
	}
</style>
