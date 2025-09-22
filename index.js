#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

const binaryName = process.platform === 'win32' ? 'gewe-notice-mcp.exe' : 'gewe-notice-mcp';
const binaryPath = path.join(__dirname, 'bin', binaryName);

// Forward all arguments and environment variables
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env,
});

child.on('error', (err) => {
  if (err.code === 'ENOENT') {
    console.error(`
Binary not found at ${binaryPath}

Please try reinstalling the package:
  npm install gewe-notice-mcp

If the problem persists, visit https://github.com/wangnov/gewe-notice-mcp for help.
`);
    process.exit(1);
  }
  console.error('Failed to start gewe-notice-mcp:', err);
  process.exit(1);
});

child.on('exit', (code) => {
  process.exit(code || 0);
});
