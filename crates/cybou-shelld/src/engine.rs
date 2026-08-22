// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Interactive Shell execution engine bound to a sandboxed filesystem.

use std::fmt::Write as _;

use cybou_jailfs::{JailError, JailFs};

use crate::types::{MAX_FILE_PAYLOAD_BYTES, MAX_WALK_DEPTH, MAX_WALK_ENTRIES, ShellOutput};

/// Interactive Shell state with virtual working directory and sandboxed filesystem.
#[derive(Clone, Debug)]
pub struct ShellEngine {
    jail: JailFs,
    virtual_cwd: String,
}

impl ShellEngine {
    /// Initialize a new shell engine bound to a sandbox root.
    #[must_use]
    pub const fn new(jail: JailFs) -> Self {
        Self {
            jail,
            virtual_cwd: String::new(),
        }
    }

    /// Return the current virtual working directory inside the sandbox.
    #[must_use]
    pub fn cwd(&self) -> &str {
        if self.virtual_cwd.is_empty() {
            "/"
        } else {
            &self.virtual_cwd
        }
    }

    /// Return a reference to the underlying sandbox filesystem.
    #[must_use]
    pub const fn jail(&self) -> &JailFs {
        &self.jail
    }

    /// Parse and execute a command string.
    #[must_use]
    pub fn execute(&mut self, command_line: &str) -> ShellOutput {
        let trimmed = command_line.trim();
        if trimmed.is_empty() {
            return ShellOutput::success(String::new(), self.cwd());
        }

        let tokens = Self::tokenize(trimmed);
        if tokens.is_empty() {
            return ShellOutput::success(String::new(), self.cwd());
        }

        let cmd = tokens[0].to_ascii_lowercase();
        let args = &tokens[1..];

        match cmd.as_str() {
            "pwd" => self.exec_pwd(),
            "cd" => self.exec_cd(args),
            "ls" => self.exec_ls(args),
            "cat" => self.exec_cat(args),
            "echo" => self.exec_echo(args),
            "stat" => self.exec_stat(args),
            "head" => self.exec_head(args),
            "tail" => self.exec_tail(args),
            "grep" => self.exec_grep(args),
            "wc" => self.exec_wc(args),
            "du" => self.exec_du(args),
            "file" => self.exec_file(args),
            "find" => self.exec_find(args),
            "date" => self.exec_date(),
            "help" => self.exec_help(args),
            "clear" => ShellOutput::success("\x1b[2J\x1b[H", self.cwd()),
            unknown => ShellOutput::error(
                127,
                format!(
                    "command not found: '{unknown}'. Type 'help' to see what this shell can do.\n"
                ),
                self.cwd(),
            ),
        }
    }

    fn resolve_path(&self, rel_or_abs: &str) -> String {
        if rel_or_abs.starts_with('/') {
            rel_or_abs.to_owned()
        } else if self.virtual_cwd.is_empty() || self.virtual_cwd == "/" {
            format!("/{rel_or_abs}")
        } else {
            format!("{}/{rel_or_abs}", self.virtual_cwd)
        }
    }

    fn exec_pwd(&self) -> ShellOutput {
        ShellOutput::success(format!("{}\n", self.cwd()), self.cwd())
    }

    fn exec_cd(&mut self, args: &[String]) -> ShellOutput {
        let target = if args.is_empty() || args[0] == "~" {
            "/".to_owned()
        } else {
            args[0].clone()
        };

        if target == ".." || target.contains("..") {
            if self.virtual_cwd.is_empty() || self.virtual_cwd == "/" {
                return ShellOutput::success(String::new(), "/");
            }
            let mut parts: Vec<&str> = self
                .virtual_cwd
                .trim_start_matches('/')
                .split('/')
                .collect();
            parts.pop();
            let new_cwd = if parts.is_empty() || parts[0].is_empty() {
                "/".to_owned()
            } else {
                format!("/{}", parts.join("/"))
            };
            self.virtual_cwd = new_cwd;
            return ShellOutput::success(String::new(), self.cwd());
        }

        let full_path = self.resolve_path(&target);
        match self.jail.resolve(&full_path) {
            Ok(resolved) => {
                if resolved.is_dir() {
                    let clean = full_path.replace('\\', "/");
                    self.virtual_cwd = if clean.is_empty() {
                        "/".to_owned()
                    } else {
                        clean
                    };
                    ShellOutput::success(String::new(), self.cwd())
                } else if resolved.exists() {
                    ShellOutput::error(1, format!("cd: not a directory: {target}\n"), self.cwd())
                } else {
                    ShellOutput::error(
                        1,
                        format!("cd: no such file or directory: {target}\n"),
                        self.cwd(),
                    )
                }
            }
            Err(JailError::TraversalAttempt(_)) => {
                ShellOutput::error(1, "cd: sandbox path traversal violation\n", self.cwd())
            }
            Err(e) => ShellOutput::error(1, format!("cd: {e}\n"), self.cwd()),
        }
    }

    fn exec_ls(&self, args: &[String]) -> ShellOutput {
        let mut show_long = false;
        let mut target_dir = self.cwd().to_owned();

        for arg in args {
            if arg == "-l" || arg == "-la" || arg == "-al" {
                show_long = true;
            } else if !arg.starts_with('-') {
                target_dir = self.resolve_path(arg);
            }
        }

        match self.jail.list_dir(&target_dir) {
            Ok(entries) => {
                let mut out = String::new();
                for entry in entries {
                    if show_long {
                        // Type, size and name, because those are what the sandbox established.
                        // This column used to read `-rwxr-xr-x 1 cybou cybou` for every entry on
                        // every host: a mode nobody read and an owner nobody looked up, printed in
                        // the format a person trusts to be a fact. A directory's size is not
                        // established either, so it is a dash rather than a plausible 4096.
                        let kind = if entry.is_dir { "dir " } else { "file" };
                        let size = if entry.is_dir {
                            "-".to_owned()
                        } else {
                            entry.size_bytes.to_string()
                        };
                        let _ = writeln!(out, "{kind} {size:>10} {}", entry.name);
                    } else {
                        let suffix = if entry.is_dir { "/" } else { "" };
                        let _ = write!(out, "{}{suffix}  ", entry.name);
                    }
                }
                if !show_long && !out.is_empty() {
                    out.push('\n');
                }
                ShellOutput::success(out, self.cwd())
            }
            Err(JailError::NotFound(p)) => ShellOutput::error(
                1,
                format!("ls: cannot access '{p}': No such file or directory\n"),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(1, format!("ls: {e}\n"), self.cwd()),
        }
    }

    fn exec_cat(&self, args: &[String]) -> ShellOutput {
        if args.is_empty() {
            return ShellOutput::error(1, "cat: missing file operand\n", self.cwd());
        }
        let full_path = self.resolve_path(&args[0]);
        match self.jail.read_to_string(&full_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(content) => ShellOutput::success(content, self.cwd()),
            Err(JailError::NotFound(p)) => ShellOutput::error(
                1,
                format!("cat: {p}: No such file or directory\n"),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(1, format!("cat: {e}\n"), self.cwd()),
        }
    }

    fn exec_echo(&self, args: &[String]) -> ShellOutput {
        let (no_newline, text_args) = if let Some(first) = args.first() {
            if first == "-n" {
                (true, &args[1..])
            } else {
                (false, args)
            }
        } else {
            (false, args)
        };
        let joined = text_args.join(" ");
        let out = if no_newline {
            joined
        } else {
            format!("{joined}\n")
        };
        ShellOutput::success(out, self.cwd())
    }

    fn exec_stat(&self, args: &[String]) -> ShellOutput {
        if args.is_empty() {
            return ShellOutput::error(1, "stat: missing operand\n", self.cwd());
        }
        let full_path = self.resolve_path(&args[0]);
        match self.jail.resolve(&full_path) {
            Ok(resolved) => {
                if let Ok(meta) = std::fs::metadata(&resolved) {
                    let file_type = if meta.is_dir() {
                        "directory"
                    } else {
                        "regular file"
                    };
                    let size = meta.len();
                    // No access line. It read `(0644/-rw-r--r--)` for everything,
                    // including directories and files whose mode was something else
                    // entirely. Permissions inside the sandbox are not something this
                    // surface establishes, and a constant printed in the position of a
                    // fact is worse than a gap.
                    let out = format!(
                        "  File: {}\n  Size: {}\t\tType: {}\n",
                        args[0], size, file_type
                    );
                    ShellOutput::success(out, self.cwd())
                } else {
                    ShellOutput::error(
                        1,
                        format!(
                            "stat: cannot stat '{}': No such file or directory\n",
                            args[0]
                        ),
                        self.cwd(),
                    )
                }
            }
            Err(JailError::TraversalAttempt(_)) => {
                ShellOutput::error(1, "stat: sandbox path traversal violation\n", self.cwd())
            }
            Err(e) => ShellOutput::error(1, format!("stat: {e}\n"), self.cwd()),
        }
    }

    fn exec_head(&self, args: &[String]) -> ShellOutput {
        let mut count = 10usize;
        let mut file_arg = None;

        let mut idx = 0;
        while idx < args.len() {
            if args[idx] == "-n" {
                if idx + 1 < args.len() {
                    count = args[idx + 1].parse().unwrap_or(10);
                    idx += 2;
                    continue;
                }
            } else if args[idx].starts_with("-n") {
                count = args[idx][2..].parse().unwrap_or(10);
            } else if !args[idx].starts_with('-') {
                file_arg = Some(&args[idx]);
            }
            idx += 1;
        }

        let Some(file_name) = file_arg else {
            return ShellOutput::error(1, "head: missing file operand\n", self.cwd());
        };

        let full_path = self.resolve_path(file_name);
        match self.jail.read_to_string(&full_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().take(count).collect();
                let mut out = lines.join("\n");
                if !out.is_empty() {
                    out.push('\n');
                }
                ShellOutput::success(out, self.cwd())
            }
            Err(JailError::NotFound(p)) => ShellOutput::error(
                1,
                format!("head: {p}: No such file or directory\n"),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(1, format!("head: {e}\n"), self.cwd()),
        }
    }

    fn exec_tail(&self, args: &[String]) -> ShellOutput {
        let mut count = 10usize;
        let mut file_arg = None;

        let mut idx = 0;
        while idx < args.len() {
            if args[idx] == "-n" {
                if idx + 1 < args.len() {
                    count = args[idx + 1].parse().unwrap_or(10);
                    idx += 2;
                    continue;
                }
            } else if args[idx].starts_with("-n") {
                count = args[idx][2..].parse().unwrap_or(10);
            } else if !args[idx].starts_with('-') {
                file_arg = Some(&args[idx]);
            }
            idx += 1;
        }

        let Some(file_name) = file_arg else {
            return ShellOutput::error(1, "tail: missing file operand\n", self.cwd());
        };

        let full_path = self.resolve_path(file_name);
        match self.jail.read_to_string(&full_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(content) => {
                let all_lines: Vec<&str> = content.lines().collect();
                let start = all_lines.len().saturating_sub(count);
                let lines = &all_lines[start..];
                let mut out = lines.join("\n");
                if !out.is_empty() {
                    out.push('\n');
                }
                ShellOutput::success(out, self.cwd())
            }
            Err(JailError::NotFound(p)) => ShellOutput::error(
                1,
                format!("tail: {p}: No such file or directory\n"),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(1, format!("tail: {e}\n"), self.cwd()),
        }
    }

    fn exec_grep(&self, args: &[String]) -> ShellOutput {
        let mut case_insensitive = false;
        let mut pattern = None;
        let mut file_arg = None;

        for arg in args {
            if arg == "-i" {
                case_insensitive = true;
            } else if pattern.is_none() {
                pattern = Some(arg.as_str());
            } else if file_arg.is_none() {
                file_arg = Some(arg.as_str());
            }
        }

        let (Some(pat), Some(file_name)) = (pattern, file_arg) else {
            return ShellOutput::error(1, "grep: usage: grep [-i] <pattern> <file>\n", self.cwd());
        };

        let full_path = self.resolve_path(file_name);
        match self.jail.read_to_string(&full_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(content) => {
                let mut matched_lines = Vec::new();
                let pat_lower = pat.to_lowercase();
                for line in content.lines() {
                    let matches = if case_insensitive {
                        line.to_lowercase().contains(&pat_lower)
                    } else {
                        line.contains(pat)
                    };
                    if matches {
                        matched_lines.push(line);
                    }
                }
                if matched_lines.is_empty() {
                    ShellOutput::error(1, "", self.cwd())
                } else {
                    let mut out = matched_lines.join("\n");
                    out.push('\n');
                    ShellOutput::success(out, self.cwd())
                }
            }
            Err(JailError::NotFound(p)) => ShellOutput::error(
                2,
                format!("grep: {p}: No such file or directory\n"),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(2, format!("grep: {e}\n"), self.cwd()),
        }
    }

    /// Count what a file actually contains.
    fn exec_wc(&self, args: &[String]) -> ShellOutput {
        let Some(target) = args.iter().find(|arg| !arg.starts_with('-')) else {
            return ShellOutput::error(
                1,
                "wc: missing file operand
",
                self.cwd(),
            );
        };
        let full_path = self.resolve_path(target);
        match self.jail.read_to_string(&full_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(content) => {
                let lines = content.lines().count();
                let words = content.split_whitespace().count();
                let bytes = content.len();
                ShellOutput::success(
                    format!(
                        "{lines} {words} {bytes} {target}
"
                    ),
                    self.cwd(),
                )
            }
            Err(JailError::NotFound(p)) => ShellOutput::error(
                1,
                format!(
                    "wc: {p}: No such file or directory
"
                ),
                self.cwd(),
            ),
            Err(e) => ShellOutput::error(
                1,
                format!(
                    "wc: {e}
"
                ),
                self.cwd(),
            ),
        }
    }

    /// Add up the sizes the sandbox reported, and say when it stopped adding.
    fn exec_du(&self, args: &[String]) -> ShellOutput {
        let target = args
            .iter()
            .find(|arg| !arg.starts_with('-'))
            .map_or_else(|| self.cwd().to_owned(), |arg| self.resolve_path(arg));

        if self.jail.is_file(&target) {
            return match self.jail.read_bytes(&target, MAX_FILE_PAYLOAD_BYTES) {
                Ok(bytes) => ShellOutput::success(
                    format!(
                        "{} {target}
",
                        bytes.len()
                    ),
                    self.cwd(),
                ),
                Err(e) => ShellOutput::error(
                    1,
                    format!(
                        "du: {e}
"
                    ),
                    self.cwd(),
                ),
            };
        }

        let mut total: u64 = 0;
        let mut visited = 0usize;
        let complete = self.walk(&target, 0, &mut visited, &mut |_, entry, _| {
            total = total.saturating_add(entry.size_bytes);
        });

        let mut out = format!(
            "{total} {target}
"
        );
        if !complete {
            // Partial is not a smaller directory. A total that stopped early says so.
            let _ = writeln!(
                out,
                "du: stopped after {visited} entries or {MAX_WALK_DEPTH} levels; this total is partial"
            );
        }
        ShellOutput::success(out, self.cwd())
    }

    /// Say what kind of thing a path is, and nothing about what it means.
    ///
    /// A directory, or a file that is valid UTF-8 text, or a file that is not. No guess at a format
    /// from an extension or a magic number: this reports what was checked, which is exactly two
    /// things.
    fn exec_file(&self, args: &[String]) -> ShellOutput {
        let Some(target) = args.iter().find(|arg| !arg.starts_with('-')) else {
            return ShellOutput::error(
                1,
                "file: missing operand
",
                self.cwd(),
            );
        };
        let full_path = self.resolve_path(target);
        if !self.jail.exists(&full_path) {
            return ShellOutput::error(
                1,
                format!(
                    "file: {target}: No such file or directory
"
                ),
                self.cwd(),
            );
        }
        if !self.jail.is_file(&full_path) {
            return ShellOutput::success(
                format!(
                    "{target}: directory
"
                ),
                self.cwd(),
            );
        }
        let kind = match self.jail.read_bytes(&full_path, MAX_FILE_PAYLOAD_BYTES) {
            Ok(bytes) if std::str::from_utf8(&bytes).is_ok() => "UTF-8 text",
            Ok(_) => "data (not UTF-8 text)",
            Err(JailError::SizeLimitExceeded { .. }) => "file too large to inspect",
            Err(e) => {
                return ShellOutput::error(
                    1,
                    format!(
                        "file: {e}
"
                    ),
                    self.cwd(),
                );
            }
        };
        ShellOutput::success(
            format!(
                "{target}: {kind}
"
            ),
            self.cwd(),
        )
    }

    /// List what is under a directory, to a bounded depth.
    fn exec_find(&self, args: &[String]) -> ShellOutput {
        let target = args
            .iter()
            .find(|arg| !arg.starts_with('-'))
            .map_or_else(|| self.cwd().to_owned(), |arg| self.resolve_path(arg));

        let mut out = String::new();
        let mut visited = 0usize;
        let complete = self.walk(&target, 0, &mut visited, &mut |path, entry, is_dir| {
            let suffix = if is_dir { "/" } else { "" };
            let _ = writeln!(out, "{path}/{}{suffix}", entry.name);
        });
        if !complete {
            let _ = writeln!(
                out,
                "find: stopped after {visited} entries or {MAX_WALK_DEPTH} levels; this listing is partial"
            );
        }
        ShellOutput::success(out, self.cwd())
    }

    /// The current instant, in UTC, from the host clock.
    ///
    /// A real reading, not a constant. `uname` was withdrawn for answering with something compiled
    /// in; a clock that did the same would be the same fault wearing a different name.
    fn exec_date(&self) -> ShellOutput {
        let now = time::OffsetDateTime::now_utc();
        let rendered = now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unavailable".to_owned());
        ShellOutput::success(
            format!(
                "{rendered}
"
            ),
            self.cwd(),
        )
    }

    /// Walk a directory, calling `visit` for each entry, within the declared bounds.
    ///
    /// Returns whether the walk finished. A caller that ignored this would report a partial answer
    /// as a complete one, which for a listing means a smaller directory and for a total means a
    /// smaller number.
    fn walk(
        &self,
        path: &str,
        depth: usize,
        visited: &mut usize,
        visit: &mut impl FnMut(&str, &cybou_jailfs::DirEntry, bool),
    ) -> bool {
        if depth >= MAX_WALK_DEPTH {
            return false;
        }
        let Ok(entries) = self.jail.list_dir(path) else {
            return true;
        };
        let mut complete = true;
        for entry in entries {
            if *visited >= MAX_WALK_ENTRIES {
                return false;
            }
            *visited += 1;
            let is_dir = entry.is_dir;
            let child = format!("{}/{}", path.trim_end_matches('/'), entry.name);
            visit(path.trim_end_matches('/'), &entry, is_dir);
            if is_dir && !self.walk(&child, depth + 1, visited, visit) {
                complete = false;
            }
        }
        complete
    }

    fn exec_help(&self, args: &[String]) -> ShellOutput {
        if let Some(cmd) = args.first() {
            let desc = match cmd.as_str() {
                "pwd" => "pwd - print name of current virtual directory",
                "cd" => "cd <dir> - change virtual working directory inside sandbox",
                "ls" => "ls [-l] [dir] - list directory contents",
                "cat" => "cat <file> - concatenate and display file content",
                "echo" => "echo [-n] [text...] - display a line of text",
                "stat" => "stat <file> - display the file type and size the sandbox established",
                "head" => "head [-n N] <file> - output the first part of files",
                "tail" => "tail [-n N] <file> - output the last part of files",
                "grep" => "grep [-i] <pattern> <file> - print lines matching a pattern",
                "wc" => "wc <file> - count lines, words and bytes",
                "du" => {
                    "du [path] - add up the sizes the sandbox reports, and say if it stopped early"
                }
                "file" => {
                    "file <path> - directory, UTF-8 text, or data; nothing guessed from the name"
                }
                "find" => "find [path] - list what is underneath, to a bounded depth",
                "date" => "date - the current instant in UTC, read from the host clock",
                "clear" => "clear - clear terminal screen buffer",
                "help" => "help [command] - display information about builtin capabilities",
                _ => "Unknown command. Type 'help' to list available capabilities.",
            };
            return ShellOutput::success(format!("{desc}\n"), self.cwd());
        }

        let help = "\
A bounded, read-only view of one directory. Available commands:

  pwd                   Print working directory
  cd <dir>              Change directory inside sandbox
  ls [-l] [dir]         List directory contents; -l adds type and size
  cat <file>            Display file contents
  echo [-n] [text...]   Display text
  stat <file>           Display the file type and size
  head [-n N] <file>    Output the first N lines (default 10)
  tail [-n N] <file>    Output the last N lines (default 10)
  grep [-i] <pat> <file> Search for pattern in file
  wc <file>             Count lines, words and bytes
  du [path]             Total size, and whether the total is partial
  file <path>           Directory, UTF-8 text, or data
  find [path]           List what is underneath, to a bounded depth
  date                  The current instant, UTC, from the host clock
  clear                 Clear screen
  help [cmd]            Display command help
";
        ShellOutput::success(help, self.cwd())
    }

    /// Split a command line string into tokens respecting single/double quotes.
    #[must_use]
    pub fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }
                ' ' | '\t' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                }
                '\\' if !in_single_quote => {
                    if let Some(next_ch) = chars.next() {
                        current.push(next_ch);
                    }
                }
                c => current.push(c),
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }
}
