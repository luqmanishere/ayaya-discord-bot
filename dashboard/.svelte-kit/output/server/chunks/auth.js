import { w as writable } from "./index.js";
const API_BASE = "/api";
class ApiClient {
  token = null;
  constructor() {
    if (typeof window !== "undefined") {
      this.token = localStorage.getItem("dashboard_token");
    }
  }
  setToken(token) {
    this.token = token;
    if (typeof window !== "undefined") {
      localStorage.setItem("dashboard_token", token);
    }
  }
  clearToken() {
    this.token = null;
    if (typeof window !== "undefined") {
      localStorage.removeItem("dashboard_token");
    }
  }
  getToken() {
    return this.token;
  }
  async request(endpoint, options = {}) {
    const headers = {
      "Content-Type": "application/json",
      ...options.headers
    };
    if (this.token) {
      headers["Authorization"] = `Bearer ${this.token}`;
    }
    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers
    });
    if (!response.ok) {
      if (response.status === 401) {
        this.clearToken();
        throw new Error("Unauthorized");
      }
      throw new Error(`API error: ${response.status} ${response.statusText}`);
    }
    return response.json();
  }
  async getAuthStatus() {
    return this.request("/auth/me");
  }
}
const api = new ApiClient();
function createAuthStore() {
  const { subscribe, set, update } = writable({
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
    login(token) {
      api.setToken(token);
      this.checkAuth();
    },
    logout() {
      api.clearToken();
      set({ isAuthenticated: false, userId: null, isLoading: false });
    }
  };
}
const auth = createAuthStore();
export {
  auth as a
};
