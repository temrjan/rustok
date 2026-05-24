#!/usr/bin/env node
const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

const VERSION = require('./package.json').version;
const BIN_DIR = path.join(__dirname, 'bin');
const BIN_PATH = path.join(BIN_DIR, 'rustok-agent-mcp');

function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();
  
  if (platform === 'darwin' && arch === 'arm64') return 'aarch64-darwin';
  if (platform === 'darwin' && arch === 'x64') return 'x86_64-darwin';
  if (platform === 'linux' && arch === 'x64') return 'x86_64-linux';
  if (platform === 'win32' && arch === 'x64') return 'x86_64-windows';
  
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        download(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed: ${response.statusCode}`));
        return;
      }
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', reject);
  });
}

async function main() {
  if (fs.existsSync(BIN_PATH)) {
    console.log('Binary already exists, skipping download.');
    return;
  }

  const platform = getPlatform();
  const ext = platform.includes('windows') ? '.zip' : '.tar.gz';
  const assetName = `rustok-agent-mcp-${platform}${ext}`;
  const url = `https://github.com/temrjan/rustok/releases/download/v${VERSION}/${assetName}`;

  console.log(`Downloading rustok-agent-mcp ${VERSION} for ${platform}...`);

  fs.mkdirSync(BIN_DIR, { recursive: true });
  const tmpPath = path.join(BIN_DIR, `download-${Date.now()}${ext}`);

  try {
    await download(url, tmpPath);

    if (ext === '.zip') {
      execSync(`unzip -o "${tmpPath}" -d "${BIN_DIR}"`, { stdio: 'inherit' });
    } else {
      execSync(`tar -xzf "${tmpPath}" -C "${BIN_DIR}"`, { stdio: 'inherit' });
    }

    fs.unlinkSync(tmpPath);
    fs.chmodSync(BIN_PATH, 0o755);
    console.log('Installation complete.');
  } catch (err) {
    console.error('Installation failed:', err.message);
    console.error('You can manually download from:', `https://github.com/temrjan/rustok/releases/tag/v${VERSION}`);
    process.exit(1);
  }
}

main();
