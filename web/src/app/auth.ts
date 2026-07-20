// Small app-local account gate. It keeps feature components independent from
// the shell while ensuring account-only actions open a real sign-in flow.

type AccountGateListener = () => void;

const listeners = new Set<AccountGateListener>();

export function requestAccountSignIn(): void {
  for (const listener of listeners) listener();
}

export function subscribeAccountGate(listener: AccountGateListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
