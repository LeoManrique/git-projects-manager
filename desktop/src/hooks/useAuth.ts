import { useCallback, useEffect, useState } from 'react';
import { api } from '../lib/api';
import { SyncUser } from '../types';

export type AuthStatus = 'loading' | 'signed-out' | 'signed-in';

interface UseAuthReturn {
  user: SyncUser | null;
  status: AuthStatus;
  error: string | null;
  signIn: () => Promise<void>;
  signOut: () => Promise<void>;
}

const SYNC_USER_EVENT = 'sync-user-changed';

function broadcast() {
  window.dispatchEvent(new Event(SYNC_USER_EVENT));
}

export function useAuth(): UseAuthReturn {
  const [user, setUser] = useState<SyncUser | null>(null);
  const [status, setStatus] = useState<AuthStatus>('loading');
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const u = await api.getSyncUser();
      setUser(u);
      setStatus(u ? 'signed-in' : 'signed-out');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStatus('signed-out');
    }
  }, []);

  useEffect(() => {
    refresh();
    const onChange = () => { refresh(); };
    window.addEventListener(SYNC_USER_EVENT, onChange);
    return () => window.removeEventListener(SYNC_USER_EVENT, onChange);
  }, [refresh]);

  const signIn = useCallback(async () => {
    setError(null);
    try {
      const u = await api.signInWithGoogle();
      setUser(u);
      setStatus('signed-in');
      broadcast();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      throw e;
    }
  }, []);

  const signOut = useCallback(async () => {
    setError(null);
    try {
      await api.signOut();
      setUser(null);
      setStatus('signed-out');
      broadcast();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      throw e;
    }
  }, []);

  return { user, status, error, signIn, signOut };
}
