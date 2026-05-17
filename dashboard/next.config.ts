import type { NextConfig } from "next";
import path from "path";

const isTauriBuild = process.env.TAURI_BUILD === '1';

const nextConfig: NextConfig = {
  output: 'export',
  ...(isTauriBuild ? { assetPrefix: './' } : {}),
  images: {
    unoptimized: true,
  },
  turbopack: {
    root: path.resolve(__dirname, '.'),
  },
};

export default nextConfig;
