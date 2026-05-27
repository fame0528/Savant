/**
 * Automated Updater Keypair Management
 *
 * Self-contained in src-tauri/scripts/. All paths relative to __dirname.
 * Checks if the Tauri updater signing keypair exists.
 * If not, generates one automatically and updates tauri.conf.json with the public key.
 *
 * Run: node scripts/setup-updater-keys.js
 * Or: automatically runs as part of `beforeBuildCommand` in tauri.conf.json
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// All paths relative to this script's location (src-tauri/scripts/)
const TAURI_DIR = path.resolve(__dirname, '..');
const KEYS_DIR = path.join(TAURI_DIR, '.tauri');
const PRIVATE_KEY_PATH = path.join(KEYS_DIR, 'updater.key');
const PUBLIC_KEY_PATH = path.join(KEYS_DIR, 'updater.key.pub');
const TAURI_CONFIG_PATH = path.join(TAURI_DIR, 'tauri.conf.json');

function ensureKeysExist() {
  if (!fs.existsSync(TAURI_CONFIG_PATH)) {
    console.error('[updater] tauri.conf.json not found at:', TAURI_CONFIG_PATH);
    process.exit(1);
  }

  // Check if keypair already exists
  if (fs.existsSync(PRIVATE_KEY_PATH) && fs.existsSync(PUBLIC_KEY_PATH)) {
    const pubkey = fs.readFileSync(PUBLIC_KEY_PATH, 'utf-8').trim();
    const config = JSON.parse(fs.readFileSync(TAURI_CONFIG_PATH, 'utf-8'));
    if (config.plugins?.updater?.pubkey === pubkey) {
      console.log('[updater] Signing keypair exists and config is up to date.');
      return;
    }
    console.log('[updater] Keypair exists but config pubkey is stale. Updating...');
    updateConfig(pubkey);
    return;
  }

  console.log('[updater] No signing keypair found. Generating...');

  if (!fs.existsSync(KEYS_DIR)) {
    fs.mkdirSync(KEYS_DIR, { recursive: true });
  }

  const password = process.env.TAURI_SIGNING_PASSWORD || '';
  const cmd = [
    'cargo', 'tauri', 'signer', 'generate',
    '--ci',
    '--write-keys', PRIVATE_KEY_PATH,
  ];

  if (password) {
    cmd.push('--password', password);
  }

  try {
    execSync(cmd.join(' '), {
      cwd: TAURI_DIR,
      stdio: ['pipe', 'pipe', 'pipe'],
      encoding: 'utf-8',
    });
  } catch (e) {
    console.error('[updater] Failed to generate keypair:', e.stderr || e.message);
    process.exit(1);
  }

  if (!fs.existsSync(PUBLIC_KEY_PATH)) {
    console.error('[updater] Public key file was not created.');
    process.exit(1);
  }

  const pubkey = fs.readFileSync(PUBLIC_KEY_PATH, 'utf-8').trim();
  console.log('[updater] Generated keypair. Public key:', pubkey.substring(0, 30) + '...');

  updateConfig(pubkey);

  if (!fs.existsSync(PRIVATE_KEY_PATH)) {
    console.error('[updater] Private key file was not created. Check permissions.');
    process.exit(1);
  }

  console.log('[updater] Keypair setup complete.');
}

function updateConfig(pubkey) {
  const config = JSON.parse(fs.readFileSync(TAURI_CONFIG_PATH, 'utf-8'));
  if (!config.plugins) config.plugins = {};
  if (!config.plugins.updater) config.plugins.updater = {};
  config.plugins.updater.pubkey = pubkey;

  fs.writeFileSync(TAURI_CONFIG_PATH, JSON.stringify(config, null, 2) + '\n');
  console.log('[updater] Updated tauri.conf.json with public key.');
}

ensureKeysExist();
