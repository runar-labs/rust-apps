import { writable } from 'svelte/store';
import type { Profile } from './types';
import { authStore } from '@kagi/auth';

export interface ProfileState {
  profile: Profile | null;
  loading: boolean;
  error: string | null;
}

const createProfileStore = () => {
  const { subscribe, set, update } = writable<ProfileState>({
    profile: null,
    loading: false,
    error: null
  });

  return {
    subscribe,
    fetchProfile: async () => {
      update(state => ({ ...state, loading: true, error: null }));

      try {
        const response = await fetch('/api/profile', {
          headers: {
            'Authorization': `Bearer ${authStore.token}`
          }
        });

        if (!response.ok) {
          throw new Error('Failed to fetch profile');
        }

        const profile = await response.json();
        update(state => ({ ...state, profile, loading: false }));
      } catch (e) {
        update(state => ({
          ...state,
          loading: false,
          error: e instanceof Error ? e.message : 'An error occurred'
        }));
      }
    },
    updateProfile: async (data: Partial<Profile>) => {
      update(state => ({ ...state, loading: true, error: null }));

      try {
        const response = await fetch('/api/profile', {
          method: 'PATCH',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${authStore.token}`
          },
          body: JSON.stringify(data)
        });

        if (!response.ok) {
          throw new Error('Failed to update profile');
        }

        const profile = await response.json();
        update(state => ({ ...state, profile, loading: false }));
      } catch (e) {
        update(state => ({
          ...state,
          loading: false,
          error: e instanceof Error ? e.message : 'An error occurred'
        }));
      }
    },
    reset: () => {
      set({
        profile: null,
        loading: false,
        error: null
      });
    }
  };
};

export const profileStore = createProfileStore(); 