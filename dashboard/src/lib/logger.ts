type LogLevel = 'debug' | 'info' | 'warn' | 'error';

const isDev = process.env.NODE_ENV === 'development';

const formatMessage = (level: LogLevel, context: string, message: string): string => {
  const timestamp = new Date().toISOString();
  return `[${timestamp}] [${level.toUpperCase()}] [${context}] ${message}`;
};

const log = (level: LogLevel, context: string, message: string, ...args: unknown[]) => {
  if (!isDev && level === 'debug') return;
  const formatted = formatMessage(level, context, message);
  const method = level === 'error' ? console.error : level === 'warn' ? console.warn : console.log;
  if (args.length > 0) {
    method(formatted, ...args);
  } else {
    method(formatted);
  }
};

export const logger = {
  debug: (context: string, message: string, ...args: unknown[]) => log('debug', context, message, ...args),
  info: (context: string, message: string, ...args: unknown[]) => log('info', context, message, ...args),
  warn: (context: string, message: string, ...args: unknown[]) => log('warn', context, message, ...args),
  error: (context: string, message: string, ...args: unknown[]) => log('error', context, message, ...args),
};
