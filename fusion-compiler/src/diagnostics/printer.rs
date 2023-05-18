use std::cmp;
use std::ops::Deref;

use termion::color;
use termion::color::{Color, Fg, Red, Reset, Yellow};

use crate::diagnostics::Diagnostic;
use crate::text::SourceText;

pub struct DiagnosticsPrinter<'a> {
    text: &'a SourceText,
    diagnostics: &'a [Diagnostic],
}

const PREFIX_LENGTH: usize = 8;

impl<'a> DiagnosticsPrinter<'a> {
    pub fn new(text: &'a SourceText, diagnostics: &'a [Diagnostic]) -> Self {
        Self {
            text,
            diagnostics,
        }
    }

    /// Stringifies the diagnostic.
    ///
    /// It uses the following format:
    ///
    /// let <red>x<reset> = 5;
    ///          ^
    ///          |
    ///          +-- This is the error message (<line>:<column>)
    ///
    pub fn stringify_diagnostic(&self, diagnostic: &Diagnostic) -> String {
        let start_line_index = self.text.line_index(diagnostic.span.start).expect("Invalid span start");
        let end_line_index = self.text.line_index(diagnostic.span.end).unwrap_or(start_line_index);
        let color: Box<dyn Color> = match diagnostic.severity {
            crate::diagnostics::DiagnosticSeverity::Error => Box::new(Red),
            crate::diagnostics::DiagnosticSeverity::Warning => Box::new(Yellow),
        };
        let mut result = String::new();

        // Print the code block with red highlights
        for line_index in start_line_index..=end_line_index {
            let line = self.text.get_line(line_index);
            let line_start = self.text.line_start(line_index);

            let (prefix, span, suffix) = if start_line_index == end_line_index {
                let column = diagnostic.span.start - line_start;
                self.get_text_spans(diagnostic, &line, column)
            } else if line_index == start_line_index {
                let column = diagnostic.span.start - line_start;
                let prefix = &line[..column];
                let span = &line[column..];
                (prefix, span, "")
            } else if line_index == end_line_index {
                let span_end = diagnostic.span.end - line_start;
                let (span, suffix) = line.split_at(span_end);
                ("", span, suffix)
            } else {
                ("", line, "")
            };


            result.push_str(&format!("{}{}{}{}{}\n", prefix, Fg(color.deref()), span, Fg(Reset), suffix));
        }

        let mut last_indent = 0;
        let mut first_indent = 0;
        // Print the arrows for the lines
        for line_index in start_line_index..=end_line_index {
            let line_start = self.text.line_start(line_index);
            let start_column = if line_index == start_line_index {
                diagnostic.span.start - line_start
            } else {
                0
            };
            if line_index == start_line_index {
                first_indent = start_column;
            }
            let end_column = if line_index == end_line_index {
                diagnostic.span.end - line_start
            } else {
                let line = self.text.get_line(line_index);
                line.len()
            };
            last_indent = start_column;
            let arrow_pointers = Self::format_arrows(start_column, end_column);
            result.push_str(&format!("{}\n", arrow_pointers));
        }
        let arrow_line = Self::format_arrow_line(last_indent);
        result.push_str(&format!("{}\n", arrow_line));
        // Print the error message
        let error_message = Self::format_error_message(&diagnostic, last_indent, first_indent, start_line_index);
        result.push_str(&error_message);

        result
    }

    fn format_arrows(start_column: usize, end_column: usize) -> String {
        let arrow_pointers = format!("{:indent$}{}", "", std::iter::repeat('^').take(end_column - start_column).collect::<String>(), indent = start_column);
        arrow_pointers
    }

    fn format_arrow_line(indent: usize) -> String {
        format!("{:indent$}|", "", indent = indent)
    }


    fn format_error_message(diagnostic: &Diagnostic, indent: usize, column: usize, line_index: usize) -> String {
        format!("{:indent$}+-- {} ({}:{})", "", diagnostic.message, column + 1, line_index + 1, indent = indent)
    }


    fn get_text_spans(&'a self, diagnostic: &Diagnostic, line: &'a str, column: usize) -> (&'a str, &'a str, &'a str) {
        // todo: use PREFIX_LENGTH here
        let prefix_start = 0;
        let prefix_end = column;
        let suffix_start = cmp::min(column + diagnostic.span.length(), line.len());
        let suffix_end = cmp::min(suffix_start + PREFIX_LENGTH, line.len());

        let prefix = &line[prefix_start..prefix_end];
        let span = &line[prefix_end..suffix_start];
        let suffix = &line[suffix_start..suffix_end];
        (prefix, span, suffix)
    }

    pub fn print(&self) {
        for diagnostic in self.diagnostics {
            println!("{}", self.stringify_diagnostic(diagnostic));
            println!();
        }
    }
}