import { writable } from 'svelte/store';
import type { User } from './types';

export interface AuthState {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
}

const createAuthStore = () => {
  const { subscribe, set, update } = writable<AuthState>({
    user: null,
    token: null,
    isAuthenticated: false
  });

  return {
    subscribe,
    setAuth: (user: User, token: string) => {
      set({
        user,
        token,
        isAuthenticated: true
      });
      localStorage.setItem('auth_token', token);
    },
    clearAuth: () => {
      set({
        user: null,
        token: null,
        isAuthenticated: false
      });
      localStorage.removeItem('auth_token');
    },
    initialize: () => {
      const token = localStorage.getItem('auth_token');
      if (token) {
        // TODO: Validate token and fetch user
        update(state => ({ ...state, token }));
      }
    }
  };
};

export const authStore = createAuthStore(); 