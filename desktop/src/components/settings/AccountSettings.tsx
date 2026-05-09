import { useState } from 'react';
import { useAuth } from '../../hooks';

export default function AccountSettings() {
  const { user, status, signIn, signOut } = useAuth();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const handleSignIn = async () => {
    setBusy(true);
    setError('');
    try {
      await signIn();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleSignOut = async () => {
    setBusy(true);
    setError('');
    try {
      await signOut();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  if (status === 'loading') {
    return (
      <div className="flex items-center justify-center py-8">
        <span className="w-4 h-4 border-2 border-text-muted border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error && (
        <div className="bg-accent-red/10 border border-accent-red/20 text-accent-red px-2.5 py-1.5 rounded text-xs">
          {error}
        </div>
      )}

      <section>
        <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2">
          Cloud Sync
        </h3>
        <p className="text-text-muted text-xs mb-4 leading-relaxed">
          Sign in with Google to sync your kanban board across devices. The app
          works fully without signing in &mdash; sync is opt-in.
        </p>

        {status === 'signed-in' && user ? (
          <div className="space-y-3">
            <div className="bg-dark-surface/60 border border-dark-border rounded p-3 space-y-1">
              <div className="text-xs text-text-secondary">Signed in as</div>
              <div className="text-sm text-text-primary">{user.name || user.email || user.sub}</div>
              {user.email && user.name && (
                <div className="text-xs text-text-muted">{user.email}</div>
              )}
            </div>
            <button
              onClick={handleSignOut}
              disabled={busy}
              className="w-full px-3 py-1.5 text-xs rounded border border-dark-border bg-dark-surface hover:bg-dark-borderSubtle text-text-primary transition-colors disabled:opacity-50"
            >
              {busy ? 'Signing out…' : 'Sign out'}
            </button>
          </div>
        ) : (
          <button
            onClick={handleSignIn}
            disabled={busy}
            className="w-full px-3 py-2 text-xs rounded bg-accent-blue/15 hover:bg-accent-blue/25 border border-accent-blue/40 text-accent-blue transition-colors disabled:opacity-50"
          >
            {busy ? 'Waiting for browser…' : 'Sign in with Google'}
          </button>
        )}
      </section>
    </div>
  );
}
