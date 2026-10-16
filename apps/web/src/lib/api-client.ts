import { env } from '@/config/env';
import type { ApiErrorBody } from '@/types/api/models';

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
  }
}

type Query = Record<string, string | number | boolean | undefined | null>;

export type RequestOptions = {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  body?: unknown;
  query?: Query;
  signal?: AbortSignal;
};

let onUnauthorized: (() => void) | null = null;
/** The app registers one handler that clears the query cache and routes to /signin. */
export function setUnauthorizedHandler(fn: (() => void) | null) {
  onUnauthorized = fn;
}

export function buildQuery(query?: Query): string {
  if (!query) return '';
  const sp = new URLSearchParams();
  for (const [k, v] of Object.entries(query)) {
    if (v === undefined || v === null || v === '' || v === false) continue;
    sp.set(k, String(v));
  }
  const s = sp.toString();
  return s ? `?${s}` : '';
}

async function parseError(res: Response): Promise<ApiError> {
  let code = 'http_error';
  let message = res.statusText || `Request failed (${res.status})`;
  try {
    const data = (await res.json()) as Partial<ApiErrorBody>;
    if (data.error) {
      code = data.error.code;
      message = data.error.message;
    }
  } catch {
    // Non-JSON body (proxy error page): keep the generic message.
  }
  return new ApiError(res.status, code, message);
}

/** Thin fetch wrapper: CSRF header, same-origin cookies, typed `{error}` parsing. No retries on 4xx. */
export async function api<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, query, signal } = opts;
  const headers: Record<string, string> = { Accept: 'application/json', [env.csrfHeader]: env.csrfValue };
  if (body !== undefined && !(body instanceof FormData)) headers['Content-Type'] = 'application/json';
  // Absolute URL: Node-based test runners reject relative fetch URLs.
  const res = await fetch(`${window.location.origin}${env.apiBase}${path}${buildQuery(query)}`, {
    method,
    headers,
    credentials: 'same-origin',
    signal,
    body: body === undefined ? undefined : body instanceof FormData ? body : JSON.stringify(body),
  });
  if (!res.ok) {
    const err = await parseError(res);
    if (res.status === 401) onUnauthorized?.();
    throw err;
  }
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}
