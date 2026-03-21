type Theme = 'dark' | 'light';

export const getTheme = (): Theme => {
  if (typeof window === 'undefined') return 'dark';
  const stored = localStorage.getItem('savant-theme') as Theme | null;
  if (stored) return stored;
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
};

export const setTheme = (theme: Theme) => {
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem('savant-theme', theme);
};

export const toggleTheme = () => {
  const current = getTheme();
  setTheme(current === 'dark' ? 'light' : 'dark');
};
