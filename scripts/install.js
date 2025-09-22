#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const BINARY_NAME = 'gewe-notice-mcp';

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  const supported = {
    'darwin-x64': 'darwin-x64',
    'darwin-arm64': 'darwin-arm64',
    'linux-x64': 'linux-x64',
    'win32-x64': 'win32-x64',
  };

  const key = `${platform}-${arch}`;

  if (!supported[key]) {
    console.error(`Unsupported platform: ${key}`);
    console.error('Supported platforms:', Object.keys(supported).join(', '));
    process.exit(1);
  }

  return supported[key];
}

function getBinaryName() {
  return process.platform === 'win32' ? `${BINARY_NAME}.exe` : BINARY_NAME;
}

function installBinary() {
  const platform = getPlatform();
  const binaryName = getBinaryName();
  const binDir = path.join(__dirname, '..', 'bin');
  const targetPath = path.join(binDir, binaryName);

  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const packageName = `gewe-notice-mcp-${platform}`;

  try {
    const packageDir = path.dirname(require.resolve(`${packageName}/package.json`));
    const sourcePath = path.join(packageDir, binaryName);

    if (fs.existsSync(sourcePath)) {
      console.log(`Installing ${packageName}...`);
      fs.copyFileSync(sourcePath, targetPath);

      if (process.platform !== 'win32') {
        fs.chmodSync(targetPath, 0o755);
      }

      console.log(`Successfully installed ${BINARY_NAME} for ${platform}`);
      return;
    }
  } catch (e) {
    // Optional dependency not installed, fallback to local build
  }

  console.log('Pre-built binary not found, attempting to build from source...');

  try {
    execSync('cargo --version', { stdio: 'ignore' });
    console.log('Building from source with cargo...');

    execSync('cargo build --release', {
      cwd: path.join(__dirname, '..'),
      stdio: 'inherit',
    });

    const sourcePath = path.join(__dirname, '..', 'target', 'release', binaryName);

    if (fs.existsSync(sourcePath)) {
      fs.copyFileSync(sourcePath, targetPath);

      if (process.platform !== 'win32') {
        fs.chmodSync(targetPath, 0o755);
      }

      console.log(`Successfully built and installed ${BINARY_NAME}`);
      return;
    }
  } catch (e) {
    console.error('Failed to build from source:', e.message);
  }

  console.error(`
Failed to install ${BINARY_NAME}

Please ensure one of the following:
1. Install the pre-built binary package for your platform
2. Have Rust and Cargo installed to build from source

Visit https://github.com/wangnov/gewe-notice-mcp for more information.
`);
  process.exit(1);
}

if (require.main === module) {
  installBinary();
}
