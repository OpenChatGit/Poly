use crate::ast::*;
use crate::lexer::{Token, SpannedToken};

/// Helper enum for parsing call arguments
enum CallArg {
    Positional(Expr),
    Keyword(String, Expr),
}

/// Helper to create a simple param without default
fn simple_param(name: &str) -> Param {
    Param { name: name.to_string(), default: None }
}

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            self.skip_newlines();
            if !self.is_at_end() {
                statements.push(self.parse_statement()?);
            }
        }
        
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.skip_newlines();
        
        match self.peek() {
            Some(Token::Let) => self.parse_let(),
            Some(Token::Fn) | Some(Token::Def) => self.parse_fn_def(),
            Some(Token::Class) => self.parse_class(),
            Some(Token::If) => self.parse_if(),
            Some(Token::While) => self.parse_while(),
            Some(Token::For) => self.parse_for(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::Import) => self.parse_import(),
            Some(Token::From) => self.parse_from_import(),
            Some(Token::Pass) => { self.advance(); Ok(Statement::Pass) }
            Some(Token::Break) => { self.advance(); Ok(Statement::Break) }
            Some(Token::Continue) => { self.advance(); Ok(Statement::Continue) }
            Some(Token::Try) => self.parse_try(),
            Some(Token::Raise) => self.parse_raise(),
            _ => self.parse_expr_or_assign(),
        }
    }

    fn parse_let(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'let'
        let name = self.expect_identifier()?;
        self.expect(Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(Statement::Let(name, value))
    }

    fn parse_fn_def(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'fn' or 'def'
        let name = self.expect_identifier()?;
        self.expect(Token::LParen)?;
        
        let mut params = Vec::new();
        let mut seen_default = false;
        
        if !self.check(&Token::RParen) {
            // Check for 'self' parameter
            if self.check(&Token::SelfKw) {
                self.advance();
                params.push(simple_param("self"));
                if self.check(&Token::Comma) {
                    self.advance();
                }
            }
            
            while !self.check(&Token::RParen) {
                let param_name = self.expect_identifier()?;
                
                // Check for default value: param=value
                let default = if self.check(&Token::Eq) {
                    self.advance();
                    seen_default = true;
                    Some(self.parse_expr()?)
                } else {
                    if seen_default {
                        return Err(format!("Non-default parameter '{}' follows default parameter", param_name));
                    }
                    None
                };
                
                params.push(Param { name: param_name, default });
                
                if !self.check(&Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.expect(Token::RParen)?;
        self.expect(Token::Colon)?;
        
        let body = self.parse_block()?;
        
        Ok(Statement::FnDef { name, params, body })
    }

    fn parse_class(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'class'
        let name = self.expect_identifier()?;
        
        // Optional parent class
        let parent = if self.check(&Token::LParen) {
            self.advance();
            let parent_name = self.expect_identifier()?;
            self.expect(Token::RParen)?;
            Some(parent_name)
        } else {
            None
        };
        
        self.expect(Token::Colon)?;
        
        // Parse class body (methods and attributes)
        let body = self.parse_block()?;
        
        // Extract methods from body
        let mut methods = Vec::new();
        for stmt in body {
            if let Statement::FnDef { name, params, body } = stmt {
                methods.push(Method { name, params, body });
            }
        }
        
        Ok(Statement::ClassDef { name, parent, methods })
    }

    fn parse_if(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'if'
        let condition = self.parse_expr()?;
        self.expect(Token::Colon)?;
        let then_body = self.parse_block()?;
        
        let mut elif_branches = Vec::new();
        let mut else_body = None;
        
        self.skip_newlines();
        while self.check(&Token::Elif) {
            self.advance();
            let elif_cond = self.parse_expr()?;
            self.expect(Token::Colon)?;
            let elif_body = self.parse_block()?;
            elif_branches.push((elif_cond, elif_body));
            self.skip_newlines();
        }
        
        if self.check(&Token::Else) {
            self.advance();
            self.expect(Token::Colon)?;
            else_body = Some(self.parse_block()?);
        }
        
        Ok(Statement::If {
            condition,
            then_body,
            elif_branches,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'while'
        let condition = self.parse_expr()?;
        self.expect(Token::Colon)?;
        let body = self.parse_block()?;
        
        Ok(Statement::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'for'
        let var = self.expect_identifier()?;
        self.expect(Token::In)?;
        let iter = self.parse_expr()?;
        self.expect(Token::Colon)?;
        let body = self.parse_block()?;
        
        Ok(Statement::For { var, iter, body })
    }

    fn parse_return(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'return'
        
        if self.check(&Token::Newline) || self.check(&Token::Dedent) || self.is_at_end() {
            Ok(Statement::Return(None))
        } else {
            Ok(Statement::Return(Some(self.parse_expr()?)))
        }
    }

    fn parse_import(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'import'
        let module = self.expect_identifier()?;
        Ok(Statement::Import(module))
    }

    fn parse_from_import(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'from'
        let module = self.expect_identifier()?;
        self.expect(Token::Import)?;
        
        let mut names = vec![self.expect_identifier()?];
        while self.check(&Token::Comma) {
            self.advance();
            names.push(self.expect_identifier()?);
        }
        
        Ok(Statement::FromImport(module, names))
    }

    fn parse_try(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'try'
        self.expect(Token::Colon)?;
        let try_body = self.parse_block()?;
        
        self.skip_newlines();
        self.expect(Token::Except)?;
        
        // Optional exception type
        let exception_type = if !self.check(&Token::Colon) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        
        self.expect(Token::Colon)?;
        let except_body = self.parse_block()?;
        
        Ok(Statement::Try {
            try_body,
            exception_type,
            except_body,
        })
    }

    fn parse_raise(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'raise'
        let expr = self.parse_expr()?;
        Ok(Statement::Raise(expr))
    }

    fn parse_expr_or_assign(&mut self) -> Result<Statement, String> {
        let expr = self.parse_expr()?;
        
        // Check for assignment operators
        if self.check(&Token::Eq) {
            self.advance();
            let value = self.parse_expr()?;
            
            match expr {
                Expr::Identifier(name) => Ok(Statement::Assign(name, value)),
                Expr::Index(target, index) => Ok(Statement::IndexAssign(*target, *index, value)),
                Expr::Attribute(target, attr) => Ok(Statement::AttrAssign(*target, attr, value)),
                _ => Err("Invalid assignment target".to_string()),
            }
        } else if self.check(&Token::PlusEq) || self.check(&Token::MinusEq) ||
                  self.check(&Token::StarEq) || self.check(&Token::SlashEq) {
            let op = match self.peek() {
                Some(Token::PlusEq) => BinOp::Add,
                Some(Token::MinusEq) => BinOp::Sub,
                Some(Token::StarEq) => BinOp::Mul,
                Some(Token::SlashEq) => BinOp::Div,
                _ => unreachable!(),
            };
            self.advance();
            let rhs = self.parse_expr()?;
            
            if let Expr::Identifier(name) = expr {
                let new_value = Expr::BinaryOp(
                    Box::new(Expr::Identifier(name.clone())),
                    op,
                    Box::new(rhs),
                );
                Ok(Statement::Assign(name, new_value))
            } else {
                Err("Invalid compound assignment target".to_string())
            }
        } else {
            Ok(Statement::Expr(expr))
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        // Expect newline after colon
        self.skip_newlines();
        
        // Expect INDENT
        if !self.check(&Token::Indent) {
            // Single-line block (no indent)
            if !self.is_at_end() && !self.check(&Token::Newline) && !self.check(&Token::Dedent) {
                statements.push(self.parse_statement()?);
            }
            return Ok(statements);
        }
        
        self.advance(); // consume INDENT
        
        // Parse statements until DEDENT
        while !self.is_at_end() && !self.check(&Token::Dedent) {
            self.skip_newlines();
            
            if self.check(&Token::Dedent) || self.is_at_end() {
                break;
            }
            
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }
        
        // Consume DEDENT if present
        if self.check(&Token::Dedent) {
            self.advance();
        }
        
        Ok(statements)
    }

    // Expression parsing with precedence
    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let expr = self.parse_or()?;
        
        // Python-style: value if condition else other
        if self.check(&Token::If) {
            self.advance();
            let condition = self.parse_or()?;
            self.expect(Token::Else)?;
            let else_expr = self.parse_ternary()?;
            return Ok(Expr::Ternary(Box::new(condition), Box::new(expr), Box::new(else_expr)));
        }
        
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        
        while self.check(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        
        while self.check(&Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp(Box::new(left), BinOp::And, Box::new(right));
        }
        
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        
        loop {
            let op = match self.peek() {
                Some(Token::EqEq) => BinOp::Eq,
                Some(Token::NotEq) => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;
        
        loop {
            let op = match self.peek() {
                Some(Token::Lt) => BinOp::Lt,
                Some(Token::Gt) => BinOp::Gt,
                Some(Token::LtEq) => BinOp::LtEq,
                Some(Token::GtEq) => BinOp::GtEq,
                Some(Token::In) => BinOp::In,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;
        
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;
        
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::SlashSlash) => BinOp::FloorDiv,
                Some(Token::Percent) => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }
        
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;
        
        if self.check(&Token::StarStar) {
            self.advance();
            let exp = self.parse_power()?; // Right associative
            return Ok(Expr::BinaryOp(Box::new(base), BinOp::Pow, Box::new(exp)));
        }
        
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Minus) => {
                self.advance();
                Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(self.parse_unary()?)))
            }
            Some(Token::Not) => {
                self.advance();
                Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(self.parse_unary()?)))
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        
        loop {
            if self.check(&Token::LParen) {
                self.advance();
                let mut args = Vec::new();
                let mut kwargs = Vec::new();
                
                if !self.check(&Token::RParen) {
                    // Parse first argument
                    let first = self.parse_call_arg()?;
                    match first {
                        CallArg::Positional(e) => args.push(e),
                        CallArg::Keyword(k, v) => kwargs.push((k, v)),
                    }
                    
                    while self.check(&Token::Comma) {
                        self.advance();
                        if self.check(&Token::RParen) { break; }
                        let arg = self.parse_call_arg()?;
                        match arg {
                            CallArg::Positional(e) => {
                                if !kwargs.is_empty() {
                                    return Err("Positional argument after keyword argument".to_string());
                                }
                                args.push(e);
                            }
                            CallArg::Keyword(k, v) => kwargs.push((k, v)),
                        }
                    }
                }
                self.expect(Token::RParen)?;
                
                // Check for UI widget syntax: Widget(...):
                if self.check(&Token::Colon) {
                    if let Expr::Identifier(widget_type) = expr {
                        // This is a UI widget with children
                        self.advance(); // consume ':'
                        let children = self.parse_widget_children()?;
                        
                        // Convert args to props
                        let mut props = kwargs;
                        // First positional arg is often the main content
                        if !args.is_empty() {
                            if let Expr::String(s) = &args[0] {
                                props.insert(0, ("text".to_string(), Expr::String(s.clone())));
                            }
                        }
                        
                        expr = Expr::Widget {
                            widget_type,
                            props,
                            children,
                        };
                        continue;
                    }
                }
                
                if kwargs.is_empty() {
                    expr = Expr::Call(Box::new(expr), args);
                } else {
                    expr = Expr::CallWithKwargs(Box::new(expr), args, kwargs);
                }
            } else if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(Token::RBracket)?;
                expr = Expr::Index(Box::new(expr), Box::new(index));
            } else if self.check(&Token::Dot) {
                self.advance();
                let attr = self.expect_identifier()?;
                expr = Expr::Attribute(Box::new(expr), attr);
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    /// Parse a single call argument (positional or keyword)
    fn parse_call_arg(&mut self) -> Result<CallArg, String> {
        // Check if this is a keyword argument: name=value
        if let Some(Token::Identifier(name)) = self.peek() {
            let name = name.clone();
            let saved_pos = self.pos;
            self.advance();
            
            if self.check(&Token::Eq) {
                self.advance();
                let value = self.parse_expr()?;
                return Ok(CallArg::Keyword(name, value));
            } else {
                // Not a keyword arg, restore position
                self.pos = saved_pos;
            }
        }
        
        Ok(CallArg::Positional(self.parse_expr()?))
    }
    
    /// Parse UI widget children (indented block of widgets)
    fn parse_widget_children(&mut self) -> Result<Vec<Expr>, String> {
        let mut children = Vec::new();
        
        self.skip_newlines();
        
        // Check for INDENT (block of children)
        if self.check(&Token::Indent) {
            self.advance();
            
            while !self.is_at_end() && !self.check(&Token::Dedent) {
                self.skip_newlines();
                if self.check(&Token::Dedent) || self.is_at_end() {
                    break;
                }
                
                let child = self.parse_expr()?;
                children.push(child);
                self.skip_newlines();
            }
            
            if self.check(&Token::Dedent) {
                self.advance();
            }
        }
        
        Ok(children)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::None) => {
                self.advance();
                Ok(Expr::None)
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Some(Token::Integer(n)) => {
                let n = *n;
                self.advance();
                Ok(Expr::Int(n))
            }
            Some(Token::Float(f)) => {
                let f = *f;
                self.advance();
                Ok(Expr::Float(f))
            }
            Some(Token::String(s)) | Some(Token::StringSingle(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String(s))
            }
            Some(Token::MultiLineString(s)) | Some(Token::MultiLineStringSingle(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String(s))
            }
            Some(Token::FString(s)) | Some(Token::FStringSingle(s)) => {
                let s = s.clone();
                self.advance();
                self.parse_fstring(&s)
            }
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Identifier(name))
            }
            Some(Token::SelfKw) => {
                self.advance();
                Ok(Expr::Identifier("self".to_string()))
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::LBracket) => self.parse_list(),
            Some(Token::LBrace) => self.parse_dict(),
            Some(Token::Lambda) => self.parse_lambda(),
            _ => Err(format!("Unexpected token: {:?}", self.peek())),
        }
    }
    
    /// Parse f-string content into parts
    fn parse_fstring(&mut self, content: &str) -> Result<Expr, String> {
        let mut parts = Vec::new();
        let mut current_literal = String::new();
        let mut chars = content.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '{' {
                // Check for escaped brace {{
                if chars.peek() == Some(&'{') {
                    chars.next();
                    current_literal.push('{');
                    continue;
                }
                
                // Save current literal if any
                if !current_literal.is_empty() {
                    parts.push(FStringPart::Literal(current_literal.clone()));
                    current_literal.clear();
                }
                
                // Extract expression until }
                let mut expr_str = String::new();
                let mut brace_depth = 1;
                
                while let Some(c) = chars.next() {
                    if c == '{' {
                        brace_depth += 1;
                        expr_str.push(c);
                    } else if c == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            break;
                        }
                        expr_str.push(c);
                    } else {
                        expr_str.push(c);
                    }
                }
                
                // Parse the expression
                let lexer = crate::lexer::Lexer::new(&expr_str);
                let tokens = lexer.tokenize();
                let mut parser = Parser::new(tokens);
                let expr = parser.parse_expr()?;
                parts.push(FStringPart::Expr(Box::new(expr)));
            } else if c == '}' {
                // Check for escaped brace }}
                if chars.peek() == Some(&'}') {
                    chars.next();
                    current_literal.push('}');
                } else {
                    current_literal.push(c);
                }
            } else {
                current_literal.push(c);
            }
        }
        
        // Add remaining literal
        if !current_literal.is_empty() {
            parts.push(FStringPart::Literal(current_literal));
        }
        
        Ok(Expr::FString(parts))
    }

    fn parse_list(&mut self) -> Result<Expr, String> {
        self.advance(); // consume '['
        let mut items = Vec::new();
        
        self.skip_newlines(); // Allow newline after '['
        
        if !self.check(&Token::RBracket) {
            items.push(self.parse_expr()?);
            
            // Check for list comprehension
            if self.check(&Token::For) {
                return self.parse_list_comprehension(items.pop().unwrap());
            }
            
            self.skip_newlines(); // Allow newline before comma or ']'
            while self.check(&Token::Comma) {
                self.advance();
                self.skip_newlines(); // Allow newline after comma
                if self.check(&Token::RBracket) { break; }
                items.push(self.parse_expr()?);
                self.skip_newlines(); // Allow newline before comma or ']'
            }
        }
        
        self.expect(Token::RBracket)?;
        Ok(Expr::List(items))
    }

    fn parse_list_comprehension(&mut self, expr: Expr) -> Result<Expr, String> {
        self.advance(); // consume 'for'
        let var = self.expect_identifier()?;
        self.expect(Token::In)?;
        let iter = self.parse_expr()?;
        
        let condition = if self.check(&Token::If) {
            self.advance();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        
        self.expect(Token::RBracket)?;
        
        Ok(Expr::ListComp {
            expr: Box::new(expr),
            var,
            iter: Box::new(iter),
            condition,
        })
    }

    fn parse_dict(&mut self) -> Result<Expr, String> {
        self.advance(); // consume '{'
        let mut pairs = Vec::new();
        
        self.skip_newlines(); // Allow newline after '{'
        
        if !self.check(&Token::RBrace) {
            let key = self.parse_expr()?;
            self.expect(Token::Colon)?;
            self.skip_newlines(); // Allow newline after ':'
            let value = self.parse_expr()?;
            pairs.push((key, value));
            
            self.skip_newlines(); // Allow newline before comma or '}'
            while self.check(&Token::Comma) {
                self.advance();
                self.skip_newlines(); // Allow newline after comma
                if self.check(&Token::RBrace) { break; }
                let key = self.parse_expr()?;
                self.expect(Token::Colon)?;
                self.skip_newlines(); // Allow newline after ':'
                let value = self.parse_expr()?;
                pairs.push((key, value));
                self.skip_newlines(); // Allow newline before comma or '}'
            }
        }
        
        self.expect(Token::RBrace)?;
        Ok(Expr::Dict(pairs))
    }

    fn parse_lambda(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'lambda'
        
        let mut params = Vec::new();
        if !self.check(&Token::Colon) {
            params.push(simple_param(&self.expect_identifier()?));
            while self.check(&Token::Comma) {
                self.advance();
                params.push(simple_param(&self.expect_identifier()?));
            }
        }
        
        self.expect(Token::Colon)?;
        let body = self.parse_expr()?;
        
        Ok(Expr::Lambda(params, Box::new(body)))
    }

    // Helper methods
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|t| &t.token)
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1).map(|t| &t.token)
    }

    fn check(&self, token: &Token) -> bool {
        self.peek().map(|t| std::mem::discriminant(t) == std::mem::discriminant(token)).unwrap_or(false)
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.peek()))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(format!("Expected identifier, got {:?}", self.peek())),
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&Token::Newline) {
            self.advance();
        }
    }
}
