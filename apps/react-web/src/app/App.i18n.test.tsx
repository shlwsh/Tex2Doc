import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { useSessionStore } from '../stores/session';

function renderApp(route: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: 0 } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[route]}>
        <App />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe('App i18n rendering', () => {
  beforeEach(() => {
    localStorage.clear();
    useSessionStore.setState({
      userSession: undefined,
      quickSession: undefined,
      adminSession: undefined,
      theme: 'light',
      locale: 'zh-CN',
    });
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
    vi.stubGlobal(
      'ResizeObserver',
      class ResizeObserver {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
    vi.stubGlobal('getComputedStyle', () => ({
      getPropertyValue: () => '',
    }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('updates shell and settings labels when locale changes', async () => {
    renderApp('/app-react/settings');

    expect(screen.getAllByText('设置').length).toBeGreaterThan(0);
    expect(screen.getByText('主题')).toBeInTheDocument();

    act(() => {
      useSessionStore.getState().setPreferences('light', 'en-US');
    });

    expect((await screen.findAllByText('Settings')).length).toBeGreaterThan(0);
    expect(screen.getByText('Theme')).toBeInTheDocument();
    expect(screen.getByText('Tex2Doc User')).toBeInTheDocument();
  });

  it('localizes table headings, empty state, and known status values', async () => {
    useSessionStore.setState({
      userSession: {
        apiBaseUrl: 'http://127.0.0.1:2624/v1/',
        accessToken: 'token',
        user: { email: 'member@example.com' },
      },
      locale: 'en-US',
    });
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify([
            {
              id: 'r1',
              code_preview: 'CODE-***',
              package_id: 'count_3',
              quantity: 1,
              redeemed_at: '2026-06-29T00:00:00Z',
              created_at: '2026-06-29T00:00:00Z',
            },
          ]),
          { status: 200, headers: { 'content-type': 'application/json' } },
        ),
      ),
    );

    renderApp('/app-react/recharge');

    expect((await screen.findAllByText('Redeem')).length).toBeGreaterThan(0);
    await waitFor(() => expect(screen.getAllByText('Code preview').length).toBeGreaterThan(0));
    expect(screen.getAllByText('Package ID').length).toBeGreaterThan(0);
    expect(screen.queryByText('暂无数据')).not.toBeInTheDocument();
  });
});
