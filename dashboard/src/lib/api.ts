const API_BASE = '/api';

export interface AuthMeResponse {
	user_id: string;
	is_authenticated: boolean;
}

export class ApiClient {
	private token: string | null = null;

	constructor() {
		if (typeof window !== 'undefined') {
			this.token = localStorage.getItem('dashboard_token');
		}
	}

	setToken(token: string) {
		this.token = token;
		if (typeof window !== 'undefined') {
			localStorage.setItem('dashboard_token', token);
		}
	}

	clearToken() {
		this.token = null;
		if (typeof window !== 'undefined') {
			localStorage.removeItem('dashboard_token');
		}
	}

	getToken(): string | null {
		return this.token;
	}

	private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
		const headers: Record<string, string> = {
			'Content-Type': 'application/json',
			...options.headers
		};

		if (this.token) {
			headers['Authorization'] = `Bearer ${this.token}`;
		}

		const response = await fetch(`${API_BASE}${endpoint}`, {
			...options,
			headers
		});

		if (!response.ok) {
			if (response.status === 401) {
				this.clearToken();
				throw new Error('Unauthorized');
			}
			throw new Error(`API error: ${response.status} ${response.statusText}`);
		}

		return response.json();
	}

	async getAuthStatus(): Promise<AuthMeResponse> {
		return this.request<AuthMeResponse>('/auth/me');
	}
}

export const api = new ApiClient();
