import { clearToken, getToken } from '../auth/token';

export class ApiClientError extends Error {
  code?: string;
  constructor(message: string, code?: string) {
    super(message);
    this.name = 'ApiClientError';
    this.code = code;
  }
}

export interface ApiResponse<T = any> {
  data: T;
  message?: string;
  error?: {
    code: string;
    message: string;
  };
}

export async function apiRequest<T = any>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const token = getToken();
  const headers = new Headers(options.headers || {});

  if (!headers.has('Content-Type') && !(options.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json');
  }

  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  const response = await fetch(path, {
    ...options,
    headers,
  });

  const json = await response.json().catch(() => ({}));

  if (response.status === 401) {
    clearToken();
  }

  if (!response.ok) {
    const errorMsg = json?.error?.message || json?.message || `HTTP ${response.status} Error`;
    const errorCode = json?.error?.code;
    throw new ApiClientError(errorMsg, errorCode);
  }

  return (json.data !== undefined ? json.data : json) as T;
}

export async function get<T = any>(path: string): Promise<T> {
  return apiRequest<T>(path, { method: 'GET' });
}

export async function post<T = any>(path: string, body?: any): Promise<T> {
  return apiRequest<T>(path, {
    method: 'POST',
    body: body ? JSON.stringify(body) : undefined,
  });
}

export async function put<T = any>(path: string, body?: any): Promise<T> {
  return apiRequest<T>(path, {
    method: 'PUT',
    body: body ? JSON.stringify(body) : undefined,
  });
}

export async function del<T = any>(path: string): Promise<T> {
  return apiRequest<T>(path, { method: 'DELETE' });
}
