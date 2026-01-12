use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use chrono::Local;
use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use std::sync::Arc;

use crate::config::LogConfig;

/// 日志轮转写入器
pub struct RotatingFileWriter {
    config: LogConfig,
    current_file: Mutex<Option<File>>,
    current_path: Mutex<PathBuf>,
    process_name: String,
    bytes_written: Mutex<u64>,
}

impl RotatingFileWriter {
    pub fn new(config: LogConfig) -> Result<Self> {
        // 获取进程名
        let process_name = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "creskyDNS".to_string());

        // 确保日志目录存在
        if let Some(parent) = Path::new(&config.path).parent() {
            fs::create_dir_all(parent)?;
        }

        let path = PathBuf::from(&config.path);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        let bytes_written = file.metadata()?.len();

        Ok(Self {
            config,
            current_file: Mutex::new(Some(file)),
            current_path: Mutex::new(path),
            process_name,
            bytes_written: Mutex::new(bytes_written),
        })
    }

    /// 检查是否需要轮转
    fn should_rotate(&self) -> Result<bool> {
        let bytes = *self.bytes_written.lock().unwrap();
        let max_size = parse_size(&self.config.max_size)?;
        
        Ok(bytes >= max_size)
    }

    /// 执行轮转
    fn rotate(&self) -> Result<()> {
        let mut file_guard = self.current_file.lock().unwrap();
        let path_guard = self.current_path.lock().unwrap();
        let mut bytes_guard = self.bytes_written.lock().unwrap();

        // 关闭当前文件
        *file_guard = std::option::Option::None;

        // 生成备份文件名
        let now = Local::now();
        let backup_path = format!(
            "{}.{}",
            path_guard.display(),
            now.format("%Y%m%d_%H%M%S")
        );

        // 重命名当前文件
        if path_guard.exists() {
            fs::rename(&*path_guard, &backup_path)?;
        }

        // 清理旧备份
        self.cleanup_old_backups()?;

        // 创建新文件
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&*path_guard)?;

        *file_guard = Some(new_file);
        *bytes_guard = 0;

        Ok(())
    }

    /// 清理旧备份
    fn cleanup_old_backups(&self) -> Result<()> {
        let path = self.current_path.lock().unwrap();
        let parent = match path.parent() {
            Some(p) => p,
            std::option::Option::None => return Ok(()),
        };

        let filename = match path.file_name() {
            Some(f) => f.to_string_lossy(),
            std::option::Option::None => return Ok(()),
        };

        // 获取所有备份文件
        let mut backups: Vec<_> = fs::read_dir(parent)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with(filename.as_ref()) && name != filename.as_ref()
            })
            .collect();

        // 按修改时间排序
        backups.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

        // 删除超出数量的旧备份
        if backups.len() > self.config.max_backups {
            let to_remove = backups.len() - self.config.max_backups;
            for backup in backups.iter().take(to_remove) {
                let _ = fs::remove_file(backup.path());
            }
        }

        Ok(())
    }

    /// 格式化日志消息
    fn format_message(&self, level: &str, module: &str, message: &str) -> String {
        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let time = now.format("%H:%M:%S%.3f").to_string();

        self.config.format
            .replace("{date}", &date)
            .replace("{time}", &time)
            .replace("{level}", level)
            .replace("{process}", &self.process_name)
            .replace("{module}", module)
            .replace("{message}", message)
    }

    /// 写入日志
    pub fn write_log(&self, level: &str, module: &str, message: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // 检查是否需要轮转
        if self.should_rotate()? {
            self.rotate()?;
        }

        let formatted = self.format_message(level, module, message);
        let formatted_bytes = formatted.as_bytes().len() as u64 + 1; // +1 for newline
        
        let mut file_guard = self.current_file.lock().unwrap();
        let mut bytes_guard = self.bytes_written.lock().unwrap();
        
        if let Some(ref mut file) = *file_guard {
            writeln!(file, "{}", formatted)?;
            file.flush()?;
            *bytes_guard += formatted_bytes;
        }

        Ok(())
    }
}

/// 解析大小字符串（如 100MB, 1GB）
fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim().to_uppercase();
    
    if s.ends_with("GB") {
        let num: u64 = s.trim_end_matches("GB").trim().parse()?;
        Ok(num * 1024 * 1024 * 1024)
    } else if s.ends_with("MB") {
        let num: u64 = s.trim_end_matches("MB").trim().parse()?;
        Ok(num * 1024 * 1024)
    } else if s.ends_with("KB") {
        let num: u64 = s.trim_end_matches("KB").trim().parse()?;
        Ok(num * 1024)
    } else {
        s.parse().map_err(|e| anyhow::anyhow!("无法解析大小: {}", e))
    }
}

/// 自定义日志写入器（用于文件）
pub struct CustomFileWriter {
    writer: Arc<RotatingFileWriter>,
}

impl CustomFileWriter {
    pub fn new(writer: Arc<RotatingFileWriter>) -> Self {
        Self { writer }
    }
}

impl std::io::Write for CustomFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // 解析日志消息
        let message = String::from_utf8_lossy(buf);
        let message = message.trim();
        
        if message.is_empty() {
            return Ok(buf.len());
        }

        // 提取级别和模块
        let (level, module, msg) = parse_log_line(message);
        
        self.writer
            .write_log(&level, &module, msg)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// 解析日志行
fn parse_log_line(line: &str) -> (String, String, &str) {
    // tracing 格式: "2024-01-12T10:30:45.123Z  INFO module_name: message"
    // 或简化格式: "INFO module_name: message"
    
    let parts: Vec<&str> = line.split_whitespace().collect();
    
    // 查找日志级别
    let mut level_idx = std::option::Option::None;
    for (i, part) in parts.iter().enumerate() {
        let upper = part.to_uppercase();
        if upper == "TRACE" || upper == "DEBUG" || upper == "INFO" 
           || upper == "WARN" || upper == "ERROR" {
            level_idx = Some(i);
            break;
        }
    }
    
    if let Some(idx) = level_idx {
        let level = parts[idx].to_uppercase();
        
        // 查找冒号分隔的模块和消息
        if let Some(_colon_pos) = line.find(':') {
            // 找到级别后的内容
            let after_level = &line[line.find(&parts[idx]).unwrap() + parts[idx].len()..];
            if let Some(module_end) = after_level.find(':') {
                let module = after_level[..module_end].trim();
                let msg = after_level[module_end + 1..].trim();
                return (level, module.to_string(), msg);
            }
        }
        
        // 如果没有找到模块，返回整行作为消息
        let msg_start = line.find(&parts[idx]).unwrap() + parts[idx].len();
        return (level, "main".to_string(), line[msg_start..].trim());
    }
    
    // 默认
    ("INFO".to_string(), "main".to_string(), line)
}

/// 初始化日志系统
pub fn init_logging(config: &LogConfig) -> Result<()> {
    // 解析日志级别
    let level = match config.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    if !config.enabled {
        // 如果日志未启用，仅使用控制台输出
        tracing_subscriber::fmt()
            .with_max_level(level)
            .init();
        return Ok(());
    }

    // 创建轮转文件写入器
    let file_writer = Arc::new(RotatingFileWriter::new(config.clone())?);
    
    // 创建文件日志层
    let file_layer = fmt::layer()
        .with_writer(move || CustomFileWriter::new(file_writer.clone()))
        .with_ansi(false)
        .with_target(true)
        .with_level(true);

    // 创建控制台日志层
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(true)
        .with_level(true);

    // 组合层
    tracing_subscriber::registry()
        .with(file_layer.with_filter(tracing_subscriber::filter::LevelFilter::from_level(level)))
        .with(console_layer.with_filter(tracing_subscriber::filter::LevelFilter::from_level(level)))
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100MB").unwrap(), 100 * 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("500KB").unwrap(), 500 * 1024);
    }

    #[test]
    fn test_parse_log_line() {
        let (level, module, msg) = parse_log_line("INFO main: DNS 转发器启动");
        assert_eq!(level, "INFO");
        assert_eq!(module, "main");
        assert_eq!(msg, "DNS 转发器启动");
    }
}
