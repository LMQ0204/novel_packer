use crossterm::{
    cursor::{MoveTo, MoveToColumn, MoveUp},
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};

pub fn clear_previous_line(num: u16) -> io::Result<()> {
    let mut stdout = io::stdout();

    // 1. 移动光标上移一行
    execute!(stdout, MoveUp(num))?;

    // 2. 移动光标到行首
    execute!(stdout, MoveToColumn(0))?;

    // 3. 清除当前行内容
    execute!(stdout, Clear(ClearType::FromCursorDown))?;

    // 确保操作生效
    stdout.flush()?;

    Ok(())
}

// pub fn clear_all() -> io::Result<()> {
//     let mut stdout = io::stdout();
//     // execute!(stdout, MoveTo(0,0))?;
//     execute!(stdout, Clear(ClearType::FromCursorUp))?;

//     Ok(())
// }
