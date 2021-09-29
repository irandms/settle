use std::process::Command;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use regex::Regex;

use crate::io::*;
use crate::parser::{self, *};
use crate::default_system_editor;

// Find markdown-style links inside of `contents` string
fn find_links(contents: &str) -> Vec<String>
{
    let re = Regex::new(&format!(r#"\[\[(.*?)\]\]"#)).unwrap();
    re.captures_iter(&contents).par_bridge()
        .map(|cap| {
            let title = cap.get(1).map_or("", |m| m.as_str()).to_string();
            title
        })
        .collect()
}

// Find tags inside of `contents` string
fn find_tags(contents: &str) -> Vec<String>
{
    let re = Regex::new(r"#([\w/_-]+?)\s+").unwrap();
    re.captures_iter(&contents).par_bridge()
        .map(|cap| {
            let tag = cap.get(1).map_or("", |m| m.as_str()).to_string();
            tag
        })
        .collect()
}

/// Return a String containing
///
/// ```md
///
/// ## Backlinks
///
/// ```
fn backlink_header() -> String
{
    "\n\
    ## Backlinks\n\
    \n"
    .to_string()
}

/// Given a markdown paragraph, generate a context line to be put in the Backlinks section of a
/// Zettel
fn backlink_paragraph_format(s: &str) -> String
{
    format!(
        "\t* {}\n",
        s,
    )
}

/// Return a String containing a markdown reference link with the template:
///
/// ```md
/// [[<title of zettel>]]
/// ```
fn backlink_str(link: &Zettel, contexts: Vec<String>) -> String
{
    format!(
        "* [[{}]]\n\
        {}\n",
        link.title,
        contexts.join(""),
    )
}

pub struct Zettel
{
    pub title: String,
    pub links: Vec<String>,
    pub tags: Vec<String>,
}

impl Zettel
{
    /// Create a Zettel with specified `title` and `links` property.
    pub fn new(title: &str) -> Self
    {
        Zettel
        {
            title: title.to_string(),
            links: vec![],
            tags: vec![],
        }
    }

    /// Create a Zettel from a file, provided a path
    pub fn from_file(s: &str) -> Self
    {
        let title = replace_extension(s, "");
        let mut zettel = Zettel::new(&title);
        let contents = file_to_string(&zettel.filename());

        zettel.links = find_links(&contents);
        zettel.tags = find_tags(&contents);
        zettel
    }

    /// Create Zettel as a physical file on the system and open system editor on it
    pub fn create(self) -> Self
    {
        let editor = default_system_editor();
        write_to_file(&self.filename(), &self.title_header());
        self.edit(&editor);
        self
    }

    /// Generate title header line for physical file
    fn title_header(&self) -> String
    {
        format!(
            "# {}\n",
            &self.title,
        )
    }

    /// Return a string with the format "`id`(FILENAME_SEPARATOR)`title`.md"
    ///
    /// # Examples
    ///
    /// ```
    /// let FILENAME_SEPARATOR = "::";
    /// let zettel = Zettel::new("2021", "structs in rust");
    ///
    /// assert_eq!(zettel.filename(), "2021::structs_in_rust.md");
    /// ```
    pub fn filename(&self) -> String
    {
        format!("{}.md", self.title)
    }

    /// Open `editor` on current Zettel
    pub fn edit(&self, editor: &str)
    {
        Command::new(editor)
            .arg(self.filename())
            .status()
            .expect("failed to execute process");
    }

    /// Overwrite (or, if it doesn't exist, create) a `## Backlinks` section at the end of the
    /// Zettel, with the links being other Zettels
    pub fn update_backlinks_section(&self, links: &Vec<Zettel>)
    {
        let file = &self.filename();
        let contents = file_to_string(file);
        let re = Regex::new(r#"\n## Backlinks(?s:.*)\z"#).unwrap();

        let mut new_contents = re.replace(&contents, "").to_string();
        let b_header = backlink_header();
        new_contents.push_str(&b_header);

        for link in links {
            let b_contexts = Self::backlink_context(&self, link);
            let b_link = backlink_str(link, b_contexts);
            new_contents.push_str(&b_link);
        }

        write_to_file(file, &new_contents)
    }

    /// Read `link.filename()` and find the paragraphs where the current Zettel is mentioned
    fn backlink_context(&self, link: &Zettel) -> Vec<String>
    {
        let file = &link.filename();
        let contents = file_to_string(file);
        let re = Regex::new(&format!(r#"\(.*?{}?\)"#, &self.filename())).unwrap();
        parser::parse(&contents)
            .into_par_iter()
            .map(|e| {
                match e {
                    Block::Paragraph(s) => backlink_paragraph_format(&s),
                }
            })
            .filter(|e| {
                re.is_match(e)
            })
            .map(|s| {
                re.replace_all(&s, "").to_string()
            })
            .collect()
    }
}
