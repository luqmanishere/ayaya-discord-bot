import { writable } from 'svelte/store';
import { api } from '$lib/api';

export interface AuthState {
	isAuthenticated: boolean;
	userId: string | null;
	isLoading: boolean;
}

function createAuthStore() {
	const { subscribe, set, update } = writable<AuthState>({
		isAuthenticated: false,
		userId: null,
		isLoading: true
	});

	return {
		subscribe,
		async checkAuth() {
			update((state) => ({ ...state, isLoading: true }));

			const token = api.getToken();
			if (!token) {
				set({ isAuthenticated: false, userId: null, isLoading: false });
				return false;
			}

			try {
				const response = await api.getAuthStatus();
				set({
					isAuthenticated: response.is_authenticated,
					userId: response.user_id,
					isLoading: false
				});
				return response.is_authenticated;
			} catch (error) {
				set({ isAuthenticated: false, userId: null, isLoading: false });
				return false;
			}
		},
		login(token: string) {
			api.setToken(token);
			this.checkAuth();
		},
		logout() {
			api.clearToken();
			set({ isAuthenticated: false, userId: null, isLoading: false });
		}
	};
}

export const auth = createAuthStore();
