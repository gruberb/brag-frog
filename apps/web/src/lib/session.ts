import { api } from '@/lib/api-client';

/** Ends the server session. Callers clear the query cache and leave the app afterwards. */
export const logout = () => api<void>('/auth/logout', { method: 'POST' });
