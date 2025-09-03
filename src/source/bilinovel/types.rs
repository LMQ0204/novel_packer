use core::fmt;
use std::path::PathBuf;

#[derive(Default, Debug)]
pub struct BiliNovel {
    pub url: String,
    pub book_name: String,
    pub tags: Option<Tags>,
    pub nums: String,
    pub notice: String,
    pub description: String,
    pub volume: Vec<Novel>,
}

#[derive(Default, Debug)]
pub struct Novel {
    url: String,
    name: String,
    cover: Option<Image>,
    tags: Option<Tags>,
    description: String,
    chapters: Vec<Chapter>,
}

#[derive(Default, Debug)]
pub struct Image {
    url: Option<url::Url>,
    src: PathBuf
}

#[derive(Default, Debug)]
pub struct Chapter {
    url: String,
    title: String,
    context: String, //???
    image:Vec<Image>
}

#[derive(Default, Debug)]
pub struct Tags {
    pub state: String,
    pub label: Vec<String>,
    pub span: Vec<String>
}

impl Tags {
    pub fn new() -> Self {
        Tags::default()
    }
}

impl Image {
    pub fn new() -> Self {
        Image::default()
    }
}

impl Chapter {
    pub fn new() -> Self {
        Chapter::default()
    }
}

impl Novel {
    pub fn new(url: String, name: String) -> Self {
        let mut res = Novel::default();
        res.url = url;
        res.name = name;
        res
    }
}

impl BiliNovel {
    pub fn new(url: String) -> Self {
        let mut res = BiliNovel::default();
        res.url = url;
        res
    }
}
use colored::Colorize;
impl fmt::Display for BiliNovel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}\n", format!("{}",self.book_name).bright_yellow().bold())?;
        if let Some(tags) = &self.tags {
            writeln!(f, "{}", tags)?;
        }
        if !self.nums.is_empty() {
            writeln!(f, "{}\n", self.nums)?;
        }
        if !self.notice.is_empty() {
            for line in self.notice.split("\n") {
                writeln!(f, "{}", format!("{}",line).on_truecolor(160, 125, 125))?
            }
            ;
        }
        if !self.description.is_empty() {
            writeln!(f, "\n{}\n", self.description)?;
        }
        for (i, v) in self.volume.iter().enumerate() {
            writeln!(f, "[{}]\t{}", i+1, v.name)?;
        }
        Ok(())
    }
}

impl fmt::Display for Tags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}  ", format!("{}", self.state).on_truecolor(21, 164, 11))?;

        let (r, mut g1,mut g) = (151, 120, 20);
        for l in &self.label {
            write!(f, "{}  ", format!("{}", l).on_truecolor(r, g ,g))?;
            g = 151 - g1 as u8;
            g1 /= 2;
        }
        for s in &self.span {
            write!(f, "{}  ",s)?;
        }
        write!(f, "\n")?;
        Ok(())
    }
}