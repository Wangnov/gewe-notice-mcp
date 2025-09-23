#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// 版本号递增类型
const BUMP_TYPES = {
  major: (version) => {
    const [major] = version.split('.');
    return `${parseInt(major) + 1}.0.0`;
  },
  minor: (version) => {
    const [major, minor] = version.split('.');
    return `${major}.${parseInt(minor) + 1}.0`;
  },
  patch: (version) => {
    const [major, minor, patch] = version.split('.');
    return `${major}.${minor}.${parseInt(patch) + 1}`;
  },
};

// 获取命令行参数
const args = process.argv.slice(2);
const bumpType = args[0] || 'patch';
const specificVersion = args[1]; // 可选：指定具体版本号

if (!BUMP_TYPES[bumpType] && !specificVersion) {
  console.error(`
使用方法:
  npm run version:patch    # 递增补丁版本 (0.0.1 -> 0.0.2)
  npm run version:minor    # 递增次版本号 (0.0.1 -> 0.1.0)
  npm run version:major    # 递增主版本号 (0.0.1 -> 1.0.0)
  npm run version:set x.y.z # 设置指定版本号
  `);
  process.exit(1);
}

// 读取当前版本
const packageJsonPath = path.join(__dirname, '..', 'package.json');
const cargoTomlPath = path.join(__dirname, '..', 'Cargo.toml');

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
const currentVersion = packageJson.version;

// 计算新版本号
const newVersion = specificVersion || BUMP_TYPES[bumpType](currentVersion);

console.log(`📦 更新版本号: ${currentVersion} -> ${newVersion}\n`);

// 1. 更新 package.json - 主版本
console.log('📝 更新 package.json 主版本...');
packageJson.version = newVersion;

// 2. 更新 package.json - optionalDependencies 版本
console.log('📝 更新 optionalDependencies 版本...');
if (packageJson.optionalDependencies) {
  const platforms = [
    'gewe-notice-mcp-darwin-x64',
    'gewe-notice-mcp-darwin-arm64',
    'gewe-notice-mcp-linux-x64',
    'gewe-notice-mcp-win32-x64',
  ];

  platforms.forEach(platform => {
    if (packageJson.optionalDependencies[platform]) {
      packageJson.optionalDependencies[platform] = newVersion;
      console.log(`   ✓ ${platform}: ${newVersion}`);
    }
  });
}

// 写入更新后的 package.json
fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
console.log('✅ package.json 更新完成\n');

// 3. 更新 Cargo.toml
console.log('📝 更新 Cargo.toml 版本...');
let cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');
cargoToml = cargoToml.replace(
  /^version = "[^"]+"/m,
  `version = "${newVersion}"`
);
fs.writeFileSync(cargoTomlPath, cargoToml);
console.log('✅ Cargo.toml 更新完成\n');

// 4. 重新构建 Cargo.lock（确保版本同步）
console.log('🔧 更新 Cargo.lock...');
try {
  execSync('cargo update --workspace', { stdio: 'inherit' });
  console.log('✅ Cargo.lock 更新完成\n');
} catch (e) {
  console.warn('⚠️  无法更新 Cargo.lock，请手动运行 cargo update\n');
}

// 5. 显示 Git 状态
console.log('📊 更改的文件:');
try {
  execSync('git status --short', { stdio: 'inherit' });
} catch (e) {
  // 忽略 git 错误
}

console.log(`
✨ 版本号已成功更新到 ${newVersion}

下一步操作:
1. 检查更改: git diff
2. 提交更改: git commit -am "chore: bump version to ${newVersion}"
3. 创建标签: git tag v${newVersion}
4. 推送到远程: git push && git push --tags
`);

// 可选：询问是否自动执行后续操作
if (process.env.CI !== 'true' && process.stdout.isTTY) {
  const readline = require('readline');
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });

  rl.question('是否自动提交并创建标签？(y/N) ', (answer) => {
    if (answer.toLowerCase() === 'y') {
      try {
        console.log('\n🔄 自动提交和创建标签...');
        execSync(`git add -A`, { stdio: 'inherit' });
        execSync(`git commit -m "chore: bump version to ${newVersion}"`, { stdio: 'inherit' });
        execSync(`git tag v${newVersion}`, { stdio: 'inherit' });
        console.log(`\n✅ 已创建提交和标签 v${newVersion}`);
        console.log('运行 "git push && git push --tags" 来推送到远程仓库');
      } catch (e) {
        console.error('❌ 自动操作失败:', e.message);
      }
    }
    rl.close();
  });
}