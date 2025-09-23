# .github/

## workflows/release.yml (214行)

### 触发条件
```yaml
L3-7  on: push.tags.v* | workflow_dispatch
```

### Jobs依赖链
```
checks -> build -> publish -> release
```

### checks (L14-31)
```
L24-25  cargo fmt --check
L27-28  cargo clippy -- -D warnings
L30-31  cargo test --all
```

### build (L33-134)
```
L37-64  matrix平台:
        darwin-x64:   macos-13,  x86_64-apple-darwin
        darwin-arm64: macos-14,  aarch64-apple-darwin
        linux-x64:    ubuntu,    x86_64-unknown-linux-musl
        win32-x64:    windows,   x86_64-pc-windows-msvc

L79     cargo build --release --target
L85     提取版本号: python3解析package.json
L91-98  复制二进制: Windows(.exe) vs Unix(+x)
L100-117 生成package.json: name,version,os,cpu,bin
L123-126 打包: tar -czf -> dist/
L129-134 上传artifact
```

### publish (L136-188)
```
L140    条件: startsWith(github.ref,'refs/tags/')
L151-153 验证optionalDependencies版本一致性
L155-161 下载所有平台artifacts
L162-175 发布平台包: 解压->npm publish
L177-183 发布主包
L185-187 冒烟测试: npx gewe-notice-mcp --version
```

### release (L189-214)
```
L193    条件: startsWith(github.ref,'refs/tags/')
L198-203 下载artifacts
L205-213 创建GitHub Release: softprops/action-gh-release
```

## Secrets
```
NPM_TOKEN       npm发布权限
GITHUB_TOKEN    自动提供
```