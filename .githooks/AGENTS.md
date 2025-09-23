# .githooks/

## pre-commit (30行)
```
L6-7    检测.rs文件变更
L8-13   cargo fmt --check      格式检查
L15-20  cargo clippy -D warnings  静态分析
L22-27  cargo test --quiet     运行测试
```

## 启用
```bash
git config core.hooksPath .githooks
```

## 绕过
```bash
git commit --no-verify
```