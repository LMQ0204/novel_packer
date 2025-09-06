@echo off
setlocal
chcp 65001

echo 开始执行 Cargo build --release...
cargo build --release

if %ERRORLEVEL% neq 0 (
    echo 错误：Cargo 构建失败！
    pause
    exit /b 1
)

echo 构建成功，正在复制可执行文件...
copy "target\release\singlefile-rs.exe" ".\singlefile-rs.exe" /Y

if %ERRORLEVEL% neq 0 (
    echo 错误：复制可执行文件失败！
    pause
    exit /b 1
)

echo 复制完成，正在执行 cargo clean...
cargo clean

echo 所有操作执行完毕！
pause

endlocal
