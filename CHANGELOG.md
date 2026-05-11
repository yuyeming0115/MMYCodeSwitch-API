# 更新日志

## v4.1.0 (2026-05-11)

### 新增功能
- **自动备份**：关闭窗口时自动静默备份，每小时定时备份
- **备份文件管理**：设置页面新增备份列表，支持查看、恢复、删除
- **自动轮转**：保留最近7个备份文件，自动清理旧备份

### UI改进
- **工具栏精简**：参考LobeHub风格，按钮更小巧扁平（36px、无边框、hover仅变色）
- 移除状态栏，布局更简洁

### 技术细节
- 新增 `export_full_backup_internal` 内部函数，避免command序列化开销
- 新增 `get_backup_files`、`delete_backup_file`、`rotate_backup_files` API
- AppContent.vue 新增 `startHourlyBackup` 定时器

---

## v4.0.0
- 初始版本发布