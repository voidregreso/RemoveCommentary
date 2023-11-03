use std::collections::VecDeque;
use derive_more::Deref;
use std::fs::File;
use std::io::Read;

pub enum Type {
    Rust, Python, CPP, Haskell
}

#[derive(Copy, Clone, Debug)]
pub struct Comment {
    pub open_pat: &'static str,
    pub close_pat: &'static str,
    pub nests: bool,
    pub keep_close_pat: bool, // whether to still return close_pat as part of the text
    pub allow_close_pat: bool, // whether to allow close_pat without matching open_pat
}

// Single-line comments shared by multiple languages.
const SL_COMMENT: Comment = Comment {
    open_pat: "//",
    close_pat: "\n",
    nests: false,
    keep_close_pat: true,
    allow_close_pat: true,
};

// Block comments for Rust and CPP are the same, so they can be reused.
const BLOCK_COMMENT: Comment = Comment {
    open_pat: "/*",
    close_pat: "*/",
    nests: false,
    keep_close_pat: false,
    allow_close_pat: false,
};

const RUST: [Comment; 2] = [SL_COMMENT, BLOCK_COMMENT];

const CPP: [Comment; 2] = [SL_COMMENT, BLOCK_COMMENT];

const PYTHON: [Comment; 3] = [
    Comment {
        open_pat: "#",
        close_pat: "\n",
        nests: false,
        keep_close_pat: true,
        allow_close_pat: true,
    },
    // String literals for Python that can act as multi-line comments
    Comment {
        open_pat: "'''",
        close_pat: "'''",
        nests: false,
        keep_close_pat: false,
        allow_close_pat: false,
    },
    Comment {
        open_pat: "\"\"\"",
        close_pat: "\"\"\"",
        nests: false,
        keep_close_pat: false,
        allow_close_pat: false,
    },
];

const HASKELL: [Comment; 2] = [
    Comment {
        open_pat: "--",
        close_pat: "\n",
        nests: false,
        keep_close_pat: true,
        allow_close_pat: true,
    },
    Comment {
        open_pat: "{-",
        close_pat: "-}",
        nests: true,
        keep_close_pat: false,
        allow_close_pat: false,
    },
];

#[derive(Deref, Debug)]
#[repr(transparent)]
struct Buf(VecDeque<char>);

impl Buf {
    fn new(max_len: usize) -> Self {
        Self(VecDeque::with_capacity(max_len))
    }

    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    fn fill_up(&mut self, iter: &mut impl Iterator<Item = char>) {
        while !self.is_full() {
            if let Some(x) = iter.next() {
                self.0.push_back(x);
            } else {
                break;
            }
        }
    }

    fn matches(&self, pat: &str) -> bool {
        self.iter().take(pat.len()).copied().eq(pat.chars())
    }

    fn pop_front(&mut self) -> char {
        self.0.pop_front().unwrap()
    }

    fn pop_front_n(&mut self, n: usize) {
        let _ = self.0.drain(..n);
    }
}

#[derive(Debug)]
enum TriOpt<T> {
    Some(T),
    None,
    Wait,
}

impl<T> From<Option<T>> for TriOpt<T> {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(t) => TriOpt::Some(t),
            None => TriOpt::None,
        }
    }
}

pub struct WithoutComments<I: Iterator<Item = char>> {
    iter: I,
    buf: Buf,
    comments: Box<[Comment]>,
    state: Option<(usize, Option<usize>)>,
    in_string: bool, // Track whether it's within a string literal
    string_delimiter: Option<char>, // Stores the delimiter of the current string
    escape_next: bool, // For handling escaped characters
}

impl<I: Iterator<Item = char>> WithoutComments<I> {
    fn new(iter: I, comments: Box<[Comment]>, buf_len: usize) -> Self {
        Self {
            iter,
            buf: Buf::new(buf_len),
            comments,
            state: None,
            in_string: false,
            string_delimiter: None,
            escape_next: false
        }
    }

    fn next_(&mut self) -> TriOpt<char> {
        // at least one element missing from previous call
        self.buf.fill_up(&mut self.iter);

        if self.buf.is_empty() {
            return TriOpt::None;
        }

        // Check status of string
        if self.in_string {
            let current_char = self.buf.pop_front();
            // Check if the next character needs to be escaped
            if current_char == '\\' && !self.escape_next {
                self.escape_next = true;
                return TriOpt::Some(current_char);
            }
            // check if the string has ended (not an escaped delimiter)
            if Some(current_char) == self.string_delimiter && !self.escape_next {
                self.in_string = false;
                self.string_delimiter = None;
            }
            // Reset the escape state
            self.escape_next = false;
            return TriOpt::Some(current_char);
        }

        if let Some((idx, ref mut nesting)) = self.state {
            let comment = &self.comments[idx];
            let &Comment {
                open_pat,
                close_pat,
                keep_close_pat,
                ..
            } = comment;

            if self.buf.matches(close_pat) {
                if !keep_close_pat {
                    self.buf.pop_front_n(close_pat.len());
                }

                match nesting {
                    // non-nesting comment or top-level comment
                    None | Some(0) => self.state = None,
                    // nested comment
                    Some(d) => *d -= 1,
                }
            } else if let Some(depth) = nesting {
                if self.buf.matches(open_pat) {
                    // matched nesting open pattern
                    self.buf.pop_front_n(open_pat.len());
                    *depth += 1;
                } else {
                    self.buf.pop_front();
                }
            } else {
                self.buf.pop_front();
            }

            TriOpt::Wait
        } else {
            for (idx, comment) in self.comments.iter().enumerate() {
                let Comment {
                    open_pat,
                    close_pat,
                    nests,
                    allow_close_pat,
                    ..
                } = comment;

                // if it matches open pattern, open
                if self.buf.matches(open_pat) {
                    self.buf.pop_front_n(open_pat.len());

                    let nesting = match nests {
                        true => Some(0),
                        false => None,
                    };
                    self.state = Some((idx, nesting));
                    return TriOpt::Wait;
                } else if self.buf.matches(close_pat) && !*allow_close_pat {
                    // if close pattern forbidden, panic
                    panic!("Got \"{}\" without matching \"{}\"", close_pat, open_pat)
                }

                // Enter the logic for handling string state
                if let Some(&first_char) = self.buf.front() {
                    match first_char {
                        // Detects the beginning of a string
                        '"' | '\'' => {
                            self.in_string = true;
                            self.string_delimiter = Some(first_char);
                            return TriOpt::Some(self.buf.pop_front());
                        }
                        // Special handling of Python triple-quotes
                        '`' if self.buf.matches("```") => {
                            self.in_string = true;
                            self.string_delimiter = Some('`');
                            self.buf.pop_front_n(3);
                            return TriOpt::Some('`');
                        }
                        _ => {}
                    }
                }
            }

            TriOpt::Some(self.buf.pop_front())
        }
    }
}

impl<I: Iterator<Item = char>> Iterator for WithoutComments<I> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.next_() {
                TriOpt::None => return None,
                TriOpt::Some(c) => return Some(c),
                TriOpt::Wait => (),
            }
        }
    }
}

pub trait IntoWithoutComments
    where
        Self: Sized + Iterator<Item = char>,
{
    fn purge_commentaries(self, language: Box<[Comment]>) -> WithoutComments<Self> {
        let mut buf_len = 0;
        for &Comment {
            open_pat,
            close_pat,
            ..
        } in language.iter()
        {
            if open_pat.len() > buf_len {
                buf_len = open_pat.len()
            }
            if close_pat.len() > buf_len {
                buf_len = close_pat.len()
            }
        }
        assert_ne!(buf_len, 0);
        WithoutComments::new(self, language, buf_len)
    }
}

impl<I: Iterator<Item = char>> IntoWithoutComments for I {}

pub fn format_from_file(path_buf: &str, lang: Type) -> Result<String, String> {
    let mut file = File::open(path_buf).map_err(|_| "File does not exist".to_string())?;
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents).map_err(|_| "Failed to read file".to_string())?;

    let lang_config = match lang {
        Type::Rust => RUST.to_vec().into_boxed_slice(),
        Type::CPP => CPP.to_vec().into_boxed_slice(),
        Type::Python => PYTHON.to_vec().into_boxed_slice(),
        Type::Haskell => HASKELL.to_vec().into_boxed_slice(),
    };

    // Assuming `without_comments` is a method provided elsewhere.
    Ok(file_contents.chars().purge_commentaries(lang_config).collect())
}