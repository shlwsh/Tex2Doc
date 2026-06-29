export class ApiError extends Error {
  constructor(
    public status: number,
    public payload: unknown,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export function defaultApiBaseUrl(): string {
  if (typeof window !== 'undefined' && ['http:', 'https:'].includes(window.location.protocol)) {
    return new URL('/v1/', window.location.origin).toString();
  }
  return 'http://127.0.0.1:2624/v1/';
}

export function normalizeBaseUrl(baseUrl: string): string {
  const trimmed = baseUrl.trim() || defaultApiBaseUrl();
  return trimmed.endsWith('/') ? trimmed : `${trimmed}/`;
}

export class HttpClient {
  readonly baseUrl: string;

  constructor(baseUrl = defaultApiBaseUrl(), private readonly accessToken?: string) {
    this.baseUrl = normalizeBaseUrl(baseUrl);
  }

  async get<T>(path: string): Promise<T> {
    return this.request<T>('GET', path);
  }

  async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('POST', path, body);
  }

  async patch<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('PATCH', path, body);
  }

  async upload<T>(path: string, file: File): Promise<T> {
    const form = new FormData();
    form.append('file', file, file.name);
    return this.fetchJson<T>(this.resolve(path), {
      method: 'POST',
      headers: this.authHeaders(),
      body: form,
    });
  }

  async download(path: string): Promise<Blob> {
    const response = await fetch(this.resolve(path), {
      headers: this.authHeaders(),
    });
    if (!response.ok) {
      throw await this.toError(response);
    }
    return response.blob();
  }

  adminPath(path: string): string {
    const clean = path.replace(/^\//, '');
    const url = new URL(this.baseUrl);
    url.pathname = `/admin/v1/${clean}`;
    url.search = '';
    return url.toString();
  }

  async adminGet<T>(path: string, query?: Record<string, string | number | undefined>): Promise<T> {
    return this.fetchJson<T>(this.withQuery(this.adminPath(path), query), {
      headers: this.jsonHeaders(),
    });
  }

  async adminPost<T>(path: string, body?: unknown): Promise<T> {
    return this.fetchJson<T>(this.adminPath(path), {
      method: 'POST',
      headers: this.jsonHeaders(),
      body: JSON.stringify(body ?? {}),
    });
  }

  async adminPatch<T>(path: string, body?: unknown): Promise<T> {
    return this.fetchJson<T>(this.adminPath(path), {
      method: 'PATCH',
      headers: this.jsonHeaders(),
      body: JSON.stringify(body ?? {}),
    });
  }

  async adminDownload(path: string, query?: Record<string, string | number | undefined>): Promise<Blob> {
    const response = await fetch(this.withQuery(this.adminPath(path), query), {
      headers: this.authHeaders(),
    });
    if (!response.ok) {
      throw await this.toError(response);
    }
    return response.blob();
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    return this.fetchJson<T>(this.resolve(path), {
      method,
      headers: this.jsonHeaders(),
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  }

  private async fetchJson<T>(input: string, init: RequestInit): Promise<T> {
    const response = await fetch(input, init);
    if (!response.ok) {
      throw await this.toError(response);
    }
    if (response.status === 204) {
      return undefined as T;
    }
    return response.json() as Promise<T>;
  }

  private resolve(path: string): string {
    return new URL(path.replace(/^\//, ''), this.baseUrl).toString();
  }

  private withQuery(input: string, query?: Record<string, string | number | undefined>): string {
    const url = new URL(input);
    Object.entries(query ?? {}).forEach(([key, value]) => {
      if (value !== undefined && value !== '') {
        url.searchParams.set(key, String(value));
      }
    });
    return url.toString();
  }

  private jsonHeaders(): HeadersInit {
    return {
      'content-type': 'application/json',
      ...this.authHeaders(),
    };
  }

  private authHeaders(): HeadersInit {
    return this.accessToken ? { authorization: `Bearer ${this.accessToken}` } : {};
  }

  private async toError(response: Response): Promise<ApiError> {
    const text = await response.text();
    let payload: unknown = text;
    try {
      payload = text ? JSON.parse(text) : {};
    } catch {
      payload = text;
    }
    const message =
      typeof payload === 'object' && payload !== null && 'message' in payload
        ? String((payload as { message?: unknown }).message)
        : response.statusText || text || 'Request failed';
    return new ApiError(response.status, payload, message);
  }
}

export function downloadBlob(blob: Blob, fileName: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(url);
}
