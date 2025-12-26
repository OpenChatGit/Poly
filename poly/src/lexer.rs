use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t]+")]  // Skip spaces and tabs, but NOT newlines
pub enum Token {
    // Keywords
    #[token("let")]
    Let,
    #[token("fn")]
    Fn,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("elif")]
    Elif,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("return")]
    Return,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("none")]
    None,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("import")]
    Import,
    #[token("from")]
    From,
    #[token("class")]
    Class,
    #[token("self")]
    SelfKw,
    #[token("def")]
    Def,
    #[token("pass")]
    Pass,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("try")]
    Try,
    #[token("except")]
    Except,
    #[token("finally")]
    Finally,
    #[token("raise")]
    Raise,
    #[token("with")]
    With,
    #[token("as")]
    As,
    #[token("lambda")]
    Lambda,
    #[token("yield")]
    Yield,
    #[token("async")]
    Async,
    #[token("await")]
    Await,

    // Literals
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Integer(i64),
    
    // Multi-line strings are handled manually in the Lexer
    // Placeholder tokens - actual parsing done in tokenize()
    MultiLineString(String),
    MultiLineStringSingle(String),
    
    // F-strings (interpolated strings)
    #[regex(r#"f"([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[2..s.len()-1].to_string())
    })]
    FString(String),
    #[regex(r#"f'([^'\\]|\\.)*'"#, |lex| {
        let s = lex.slice();
        Some(s[2..s.len()-1].to_string())
    })]
    FStringSingle(String),
    
    // Regular strings
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    String(String),
    #[regex(r#"'([^'\\]|\\.)*'"#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    StringSingle(String),

    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("**")]
    StarStar,
    #[token("/")]
    Slash,
    #[token("//")]
    SlashSlash,
    #[token("%")]
    Percent,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("=")]
    Eq,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,
    #[token("@")]
    At,

    // Whitespace handling for Python-like indentation
    #[token("\n")]
    Newline,
    #[regex(r"#[^\n]*")]
    Comment,
    
    // Special tokens added by preprocessor
    Indent,
    Dedent,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: std::ops::Range<usize>,
    pub line: usize,
    pub column: usize,
}

/// Lexer with proper indentation tracking
pub struct Lexer<'a> {
    source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    pub fn tokenize(self) -> Vec<SpannedToken> {
        // First, preprocess to handle multi-line strings
        let processed = self.preprocess_multiline_strings();
        
        let mut raw_tokens = Vec::new();
        let mut inner = Token::lexer(&processed);
        let mut line = 1;
        let mut column = 1;
        
        // First pass: collect raw tokens
        while let Some(result) = inner.next() {
            match result {
                Ok(token) => {
                    let span = inner.span();
                    
                    if matches!(token, Token::Newline) {
                        raw_tokens.push(SpannedToken {
                            token,
                            span: span.clone(),
                            line,
                            column,
                        });
                        line += 1;
                        column = 1;
                    } else if !matches!(token, Token::Comment) {
                        raw_tokens.push(SpannedToken {
                            token,
                            span: span.clone(),
                            line,
                            column,
                        });
                        column += span.len();
                    }
                }
                Err(_) => {
                    column += 1;
                }
            }
        }
        
        // Second pass: add INDENT/DEDENT tokens
        self.add_indentation(raw_tokens, &processed)
    }
    
    /// Preprocess source to convert multi-line strings to single-line with escape sequences
    fn preprocess_multiline_strings(&self) -> String {
        let mut result = String::new();
        let mut chars = self.source.chars().peekable();
        
        while let Some(c) = chars.next() {
            // Check for triple quotes
            if c == '"' || c == '\'' {
                let quote = c;
                
                // Check for triple quote
                if chars.peek() == Some(&quote) {
                    chars.next();
                    if chars.peek() == Some(&quote) {
                        chars.next();
                        // Found triple quote - collect until closing triple quote
                        let mut content = String::new();
                        let mut found_end = false;
                        
                        while let Some(ch) = chars.next() {
                            if ch == quote {
                                if chars.peek() == Some(&quote) {
                                    chars.next();
                                    if chars.peek() == Some(&quote) {
                                        chars.next();
                                        found_end = true;
                                        break;
                                    } else {
                                        content.push(quote);
                                        content.push(quote);
                                    }
                                } else {
                                    content.push(quote);
                                }
                            } else {
                                content.push(ch);
                            }
                        }
                        
                        if found_end {
                            // Convert to regular string with escaped special chars
                            // Order matters: escape backslashes first, then other chars
                            let mut escaped = String::new();
                            let mut content_chars = content.chars().peekable();
                            
                            while let Some(ch) = content_chars.next() {
                                match ch {
                                    '\\' => {
                                        // Check if it's already an escape sequence
                                        if let Some(&next) = content_chars.peek() {
                                            match next {
                                                'n' | 'r' | 't' | '\\' | '"' | '\'' => {
                                                    // Already escaped, keep as-is
                                                    escaped.push('\\');
                                                    escaped.push(content_chars.next().unwrap());
                                                }
                                                _ => {
                                                    // Lone backslash, escape it
                                                    escaped.push_str("\\\\");
                                                }
                                            }
                                        } else {
                                            escaped.push_str("\\\\");
                                        }
                                    }
                                    '\n' => escaped.push_str("\\n"),
                                    '\r' => escaped.push_str("\\r"),
                                    '\t' => escaped.push_str("\\t"),
                                    '"' => escaped.push_str("\\\""),
                                    _ => escaped.push(ch),
                                }
                            }
                            
                            // Use double quotes for the result
                            result.push('"');
                            result.push_str(&escaped);
                            result.push('"');
                        } else {
                            // Unclosed triple quote - just output as-is
                            result.push(quote);
                            result.push(quote);
                            result.push(quote);
                            result.push_str(&content);
                        }
                        continue;
                    } else {
                        // Just two quotes - empty string
                        result.push(quote);
                        result.push(quote);
                        continue;
                    }
                }
            }
            
            result.push(c);
        }
        
        result
    }
    
    fn add_indentation(&self, raw_tokens: Vec<SpannedToken>, processed_source: &str) -> Vec<SpannedToken> {
        let mut result = Vec::new();
        let mut indent_stack: Vec<usize> = vec![0];
        let lines: Vec<&str> = processed_source.lines().collect();
        let mut i = 0;
        let mut current_line = 0;
        
        // Track bracket nesting - no indent/dedent inside brackets
        let mut bracket_depth: usize = 0;
        
        while i < raw_tokens.len() {
            let token = &raw_tokens[i];
            
            // Track bracket depth
            match &token.token {
                Token::LParen | Token::LBracket | Token::LBrace => bracket_depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => bracket_depth = bracket_depth.saturating_sub(1),
                _ => {}
            }
            
            // After a newline, check indentation (only if not inside brackets)
            if matches!(token.token, Token::Newline) {
                // Skip newlines inside brackets (implicit line continuation)
                if bracket_depth > 0 {
                    i += 1;
                    continue;
                }
                
                result.push(token.clone());
                current_line = token.line;
                
                // Look at the next non-empty line's indentation
                if i + 1 < raw_tokens.len() {
                    let next_token = &raw_tokens[i + 1];
                    
                    // Skip if next is also newline (empty line)
                    if matches!(next_token.token, Token::Newline) {
                        i += 1;
                        continue;
                    }
                    
                    // Calculate indentation of next line
                    let next_line_idx = next_token.line - 1;
                    if next_line_idx < lines.len() {
                        let line_content = lines[next_line_idx];
                        let indent = line_content.len() - line_content.trim_start().len();
                        let current_indent = *indent_stack.last().unwrap();
                        
                        if indent > current_indent {
                            // Indent
                            indent_stack.push(indent);
                            result.push(SpannedToken {
                                token: Token::Indent,
                                span: 0..0,
                                line: next_token.line,
                                column: 1,
                            });
                        } else if indent < current_indent {
                            // Dedent (possibly multiple)
                            while indent_stack.len() > 1 && *indent_stack.last().unwrap() > indent {
                                indent_stack.pop();
                                result.push(SpannedToken {
                                    token: Token::Dedent,
                                    span: 0..0,
                                    line: next_token.line,
                                    column: 1,
                                });
                            }
                        }
                    }
                }
            } else {
                result.push(token.clone());
            }
            
            i += 1;
        }
        
        // Add final dedents
        while indent_stack.len() > 1 {
            indent_stack.pop();
            result.push(SpannedToken {
                token: Token::Dedent,
                span: 0..0,
                line: current_line + 1,
                column: 1,
            });
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let source = "let x = 42";
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize().into_iter().map(|t| t.token).collect();
        
        assert_eq!(tokens, vec![
            Token::Let,
            Token::Identifier("x".to_string()),
            Token::Eq,
            Token::Integer(42),
        ]);
    }

    #[test]
    fn test_indentation() {
        let source = "if true:\n    print(1)\n    print(2)\nprint(3)";
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize().into_iter().map(|t| t.token).collect();
        
        assert!(tokens.contains(&Token::Indent));
        assert!(tokens.contains(&Token::Dedent));
    }

    #[test]
    fn test_function() {
        let source = "fn add(a, b):\n    return a + b";
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.tokenize().into_iter().map(|t| t.token).collect();
        
        assert!(tokens.contains(&Token::Fn));
        assert!(tokens.contains(&Token::Return));
        assert!(tokens.contains(&Token::Indent));
    }
}
