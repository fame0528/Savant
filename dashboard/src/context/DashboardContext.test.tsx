import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act } from '@testing-library/react';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), prefetch: vi.fn(), back: vi.fn(), forward: vi.fn() }),
  usePathname: () => '/',
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText: vi.fn().mockResolvedValue(undefined),
  readText: vi.fn().mockResolvedValue(''),
}));

vi.mock('@/lib/tauri', () => ({
  isTauri: () => false,
  igniteSwarm: vi.fn().mockResolvedValue('Swarm ignited'),
}));

vi.mock('@/lib/logger', () => ({
  logger: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() },
}));

describe('Dashboard Context', () => {
  beforeEach(() => { vi.clearAllMocks(); });

  it('should export useDashboard hook', async () => {
    const { useDashboard } = await import('@/context/DashboardContext');
    expect(useDashboard).toBeDefined();
    expect(typeof useDashboard).toBe('function');
  });

  it('should export DashboardProvider', async () => {
    const { DashboardProvider } = await import('@/context/DashboardContext');
    expect(DashboardProvider).toBeDefined();
  });

  it('should provide default state values', async () => {
    const { DashboardProvider, useDashboard } = await import('@/context/DashboardContext');
    let capturedState: ReturnType<typeof useDashboard> | null = null;
    const TestConsumer = () => { capturedState = useDashboard(); return null; };

    render(<DashboardProvider><TestConsumer /></DashboardProvider>);

    expect(capturedState).not.toBeNull();
    expect(capturedState!.agents).toEqual([]);
    expect(capturedState!.proposedMutations).toEqual([]);
    expect(capturedState!.mutationHistory).toEqual([]);
    expect(capturedState!.evolutionScore).toBeNull();
    expect(capturedState!.traitSnapshots).toEqual([]);
  });

  it('should toggle collapse state', async () => {
    const { DashboardProvider, useDashboard } = await import('@/context/DashboardContext');
    let capturedState: ReturnType<typeof useDashboard> | null = null;
    const TestConsumer = () => { capturedState = useDashboard(); return <div data-testid="c" />; };

    const { rerender } = render(<DashboardProvider><TestConsumer /></DashboardProvider>);
    expect(capturedState!.isCollapsed).toBe(false);
    await act(async () => { capturedState!.setIsCollapsed(true); });
    rerender(<DashboardProvider><TestConsumer /></DashboardProvider>);
    expect(capturedState!.isCollapsed).toBe(true);
  });

  it('should toggle debug mode', async () => {
    const { DashboardProvider, useDashboard } = await import('@/context/DashboardContext');
    let capturedState: ReturnType<typeof useDashboard> | null = null;
    const TestConsumer = () => { capturedState = useDashboard(); return <div data-testid="c" />; };

    const { rerender } = render(<DashboardProvider><TestConsumer /></DashboardProvider>);
    expect(capturedState!.showDebug).toBe(false);
    await act(async () => { capturedState!.setShowDebug(true); });
    rerender(<DashboardProvider><TestConsumer /></DashboardProvider>);
    expect(capturedState!.showDebug).toBe(true);
  });

  it('should toggle evolution mode', async () => {
    const { DashboardProvider, useDashboard } = await import('@/context/DashboardContext');
    let capturedState: ReturnType<typeof useDashboard> | null = null;
    const TestConsumer = () => { capturedState = useDashboard(); return <div data-testid="c" />; };

    const { rerender } = render(<DashboardProvider><TestConsumer /></DashboardProvider>);
    expect(capturedState!.isEvolutionMode).toBe(false);
    await act(async () => { capturedState!.setIsEvolutionMode(true); });
    rerender(<DashboardProvider><TestConsumer /></DashboardProvider>);
    expect(capturedState!.isEvolutionMode).toBe(true);
  });

  it('should have connection status', async () => {
    const { DashboardProvider, useDashboard } = await import('@/context/DashboardContext');
    let capturedState: ReturnType<typeof useDashboard> | null = null;
    const TestConsumer = () => { capturedState = useDashboard(); return null; };

    render(<DashboardProvider><TestConsumer /></DashboardProvider>);
    expect(capturedState!.connectionStatus).toBeDefined();
    expect(['NOMINAL', 'OFFLINE']).toContain(capturedState!.connectionStatus);
  });
});
