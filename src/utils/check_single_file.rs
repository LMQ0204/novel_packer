use std::env;
use std::fs;
use std::process::Command;

use tracing::{error};

use crate::utils::config::DynamicConfig;

/// 检查可执行文件的状态，返回Result
/// Ok(()) 表示存在且可运行
/// Err(&str) 包含具体错误信息："not found" 或 "exists but cannot run"
pub fn check_exe(exe_name: &str, config: DynamicConfig) -> Result<(), String> {
    // 检查文件是否存在（当前目录或PATH中）
    let exists = {
        // 检查当前目录
        if fs::metadata(exe_name).is_ok() {
            true
        } else if config.get_executable_path().is_some() {
            true
        } else {
            // 检查PATH中的目录
            let path_env = match env::var("PATH") {
                Ok(val) => val,
                Err(_) => {
                    error!("无法找到{}!", exe_name);
                    return Err(format!("无法找到{}!", exe_name))
                }
            };

            // 适配不同操作系统的路径分隔符
            #[cfg(windows)]
            let path_sep = ';';
            #[cfg(not(windows))]
            let path_sep = ':';

            let path_dirs: Vec<&str> = path_env.split(path_sep).collect();

            path_dirs.iter().any(|dir| {
                #[cfg(windows)]
                let full_path = format!("{}\\{}", dir, exe_name);
                #[cfg(not(windows))]
                let full_path = format!("{}/{}", dir, exe_name);
                fs::metadata(&full_path).is_ok()
            })
        }
    };

    // 如果不存在，返回错误
    if !exists {
        error!("找不到{}!", exe_name);
        return Err(format!("找不到{}!", exe_name));
    }

    // 尝试执行来验证是否可运行
    match Command::new(exe_name).arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),

        _ => {
            error!("{}存在但是无法正常运行!", exe_name);
            Err(format!("{}存在但是无法正常运行!", exe_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    #[cfg(not(windows))]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    #[test]
    fn test_not_found() {
        // 测试一个肯定不存在的文件
        let non_existent_exe = "this_file_should_never_exist_1234.exe";
        assert_eq!(
            check_exe(non_existent_exe, DynamicConfig::new()),
            Err(format!("找不到{}!", non_existent_exe))
        );
    }

    #[test]
    fn test_exists_but_cannot_run() {
        // 创建一个临时目录
        let temp_dir = TempDir::new().expect("无法创建临时目录");
        let temp_path = temp_dir.path();

        // 创建一个不可执行的文件
        #[cfg(windows)]
        let exe_name = "bad_executable.exe";
        #[cfg(not(windows))]
        let exe_name = "bad_executable";

        let exe_path = temp_path.join(exe_name);

        // 写入一些内容
        fs::write(&exe_path, "not an executable").expect("无法写入文件");

        // 在类Unix系统上，移除执行权限
        #[cfg(not(windows))]
        {
            let mut permissions = fs::metadata(&exe_path).unwrap().permissions();
            permissions.set_mode(0o644); // 只有读写权限，没有执行权限
            fs::set_permissions(&exe_path, permissions).unwrap();
        }

        // 将临时目录添加到PATH
        let original_path = env::var("PATH").unwrap();
        // 确定系统的路径分隔符
        #[cfg(windows)]
        let path_separator = ";";
        #[cfg(not(windows))]
        let path_separator = ":";

        // 拼接新的PATH环境变量
        let new_path = format!(
            "{}{}{}",
            temp_path.to_str().unwrap(),
            path_separator,
            original_path
        );

        unsafe {
            env::set_var("PATH", new_path);
        }

        // 检查结果
        assert_eq!(
            check_exe(exe_name, DynamicConfig::new()),
            Err(format!("{}存在但是无法正常运行!",&exe_name))
        );

        // 恢复原始PATH
        unsafe {
            env::set_var("PATH", original_path);
        }
    }

    #[test]
    fn test_exists_and_works() {
        // 测试系统中肯定存在且可运行的程序
        #[cfg(windows)]
        let test_exe = "cmd.exe"; // Windows命令行
        #[cfg(target_os = "linux")]
        let test_exe = "ls"; // Linux目录列表命令
        #[cfg(target_os = "macos")]
        let test_exe = "ls"; // macOS目录列表命令

        assert_eq!(check_exe(test_exe, DynamicConfig::new()), Ok(()));
    }
}
