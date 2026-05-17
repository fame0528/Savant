import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), prefetch: vi.fn() }),
  usePathname: () => '/',
  useSearchParams: () => new URLSearchParams(),
}));

describe('SetupWizard', () => {
  beforeEach(() => { vi.clearAllMocks(); });

  it('should render without crashing', async () => {
    const { default: SetupWizard } = await import('@/components/SetupWizard');
    const { container } = render(<SetupWizard onComplete={vi.fn()} />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should render with onComplete callback', async () => {
    const onComplete = vi.fn();
    const { default: SetupWizard } = await import('@/components/SetupWizard');
    render(<SetupWizard onComplete={onComplete} />);
    expect(onComplete).not.toHaveBeenCalled();
  });

  it('should show hardware detection initially', async () => {
    const { default: SetupWizard } = await import('@/components/SetupWizard');
    render(<SetupWizard onComplete={vi.fn()} />);
    const body = document.body.textContent || '';
    expect(body).toMatch(/Detecting|Hardware|system/i);
  });

  it('should have content after render', async () => {
    const { default: SetupWizard } = await import('@/components/SetupWizard');
    const { container } = render(<SetupWizard onComplete={vi.fn()} />);
    expect(container.textContent?.length).toBeGreaterThan(20);
  });
});
