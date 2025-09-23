# scripts/

## install.js (108行)
```
L9-29    getPlatform()     平台检测:darwin/linux/win32 + x64/arm64
L31-33   getBinaryName()   Windows加.exe后缀
L35-103  installBinary()
  L45-64   尝试预编译包    require.resolve() -> copyFileSync -> chmod 755
  L66-91   回退源码编译    cargo build --release -> target/release/
```

## bump-version.js (137行)
```
L8-21    BUMP_TYPES        major/minor/patch递增函数
L47      新版本计算        specificVersion || BUMP_TYPES[bumpType](currentVersion)
L53      package.json版本  packageJson.version = newVersion
L57-71   optionalDeps更新  4个平台包版本同步
L80-82   Cargo.toml更新    正则替换version行
L90      Cargo.lock更新    cargo update --workspace
L115-137 交互式提交        询问->git add/commit/tag
```

## NPM脚本
```json
"version:patch": "node scripts/bump-version.js patch"
"version:minor": "node scripts/bump-version.js minor"
"version:major": "node scripts/bump-version.js major"
"version:set":   "node scripts/bump-version.js set"
"postinstall":   "node scripts/install.js"
```