//! Minimal JavaScript engine for NaBrowse.
//!
//! Phase 0: a small but real lexer, recursive-descent parser, and tree-walk
//! interpreter for expressions and basic statements. This is the foundation
//! for DOM scripting; it grows over time.
//!
//! Privacy: runs entirely in-process, no network, no telemetry, no eval of
//! remote code unless the page explicitly loads it.

/// A JavaScript value.
#[derive(Debug, Clone, PartialEq)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(JsObject),
    Function(JsFunction),
}

/// A JavaScript object (string -> value map).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsObject(pub Vec<(String, JsValue)>);

impl JsObject {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn get(&self, key: &str) -> Option<&JsValue> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn set(&mut self, key: String, value: JsValue) {
        if let Some(existing) = self.0.iter_mut().find(|(k, _)| k == &key) {
            existing.1 = value;
        } else {
            self.0.push((key, value));
        }
    }
}

/// A JavaScript function.
#[derive(Debug, Clone, PartialEq)]
pub struct JsFunction {
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

/// The AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    VariableDeclaration { name: String, value: Expression },
    Block(Vec<Statement>),
    If {
        condition: Expression,
        consequent: Box<Statement>,
        alternate: Option<Box<Statement>>,
    },
    While { condition: Expression, body: Box<Statement> },
    Return(Expression),
    ExpressionStatement(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(JsValue),
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary { op: UnaryOp, operand: Box<Expression> },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    Member {
        object: Box<Expression>,
        property: String,
    },
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Plus,
    Minus,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
}

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    String(String),
    Identifier(String),
    Keyword(Keyword),
    Punctuator(Punctuator),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
enum Keyword {
    Var,
    Let,
    Const,
    Function,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Null,
    Undefined,
}

#[derive(Debug, Clone, PartialEq)]
enum Punctuator {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Semicolon,
    Comma,
    Dot,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    OrPunct,
    AndPunct,
    Bang,
}

const KEYWORDS: &[(&str, Keyword)] = &[
    ("var", Keyword::Var),
    ("let", Keyword::Let),
    ("const", Keyword::Const),
    ("function", Keyword::Function),
    ("if", Keyword::If),
    ("else", Keyword::Else),
    ("while", Keyword::While),
    ("return", Keyword::Return),
    ("true", Keyword::True),
    ("false", Keyword::False),
    ("null", Keyword::Null),
    ("undefined", Keyword::Undefined),
];

/// The JavaScript engine runtime.
pub struct JsEngine {
    globals: JsObject,
}

impl JsEngine {
    pub fn new() -> Self {
        let mut globals = JsObject::new();
        globals.set(
            "NaBrowse".to_string(),
            JsValue::Object(JsObject::new()),
        );
        Self { globals }
    }

    /// Execute a source string in the global scope.
    pub fn execute(&mut self, source: &str) -> Result<JsValue, JsError> {
        let tokens = Lexer::new(source).tokenize();
        let mut parser = Parser::new(tokens);
        let statements = parser.parse_program()?;

        let mut interpreter = Interpreter {
            globals: self.globals.clone(),
            env: Environment::new(),
        };
        let result = interpreter.execute_block(statements)?;
        self.globals = interpreter.globals;
        Ok(result)
    }
}

impl Default for JsEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Lexer: turns source text into tokens.
struct Lexer {
    chars: Vec<char>,
    position: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            position: 0,
        }
    }

    fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            match token {
                Token::Eof => {
                    tokens.push(Token::Eof);
                    break;
                }
                _ => tokens.push(token),
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        if self.position >= self.chars.len() {
            return Token::Eof;
        }

        let ch = self.chars[self.position];

        // Numbers.
        if ch.is_ascii_digit() || ch == '.' {
            return self.read_number();
        }

        // Strings.
        if ch == '"' || ch == '\'' {
            return self.read_string();
        }

        // Identifiers / keywords.
        if ch.is_ascii_alphabetic() || ch == '_' || ch == '$' {
            return self.read_identifier();
        }

        // Two-character punctuators.
        let two = self.read_two_char();
        if two.is_some() {
            return two.unwrap();
        }

        // Single-character punctuators.
        self.position += 1;
        match ch {
            '(' => Token::Punctuator(Punctuator::LeftParen),
            ')' => Token::Punctuator(Punctuator::RightParen),
            '{' => Token::Punctuator(Punctuator::LeftBrace),
            '}' => Token::Punctuator(Punctuator::RightBrace),
            ';' => Token::Punctuator(Punctuator::Semicolon),
            ',' => Token::Punctuator(Punctuator::Comma),
            '.' => Token::Punctuator(Punctuator::Dot),
            '=' => Token::Punctuator(Punctuator::Assign),
            '+' => Token::Punctuator(Punctuator::Plus),
            '-' => Token::Punctuator(Punctuator::Minus),
            '*' => Token::Punctuator(Punctuator::Star),
            '/' => Token::Punctuator(Punctuator::Slash),
            '!' => Token::Punctuator(Punctuator::Bang),
            '<' => Token::Punctuator(Punctuator::Less),
            '>' => Token::Punctuator(Punctuator::Greater),
            _ => Token::Identifier(ch.to_string()),
        }
    }

    fn read_two_char(&mut self) -> Option<Token> {
        if self.position + 1 >= self.chars.len() {
            return None;
        }
        let a = self.chars[self.position];
        let b = self.chars[self.position + 1];
        let token = match (a, b) {
            ('=', '=') => Token::Punctuator(Punctuator::EqualEqual),
            ('!', '=') => Token::Punctuator(Punctuator::BangEqual),
            ('<', '=') => Token::Punctuator(Punctuator::LessEqual),
            ('>', '=') => Token::Punctuator(Punctuator::GreaterEqual),
            ('|', '|') => Token::Punctuator(Punctuator::OrPunct),
            ('&', '&') => Token::Punctuator(Punctuator::AndPunct),
            _ => return None,
        };
        self.position += 2;
        Some(token)
    }

    fn read_number(&mut self) -> Token {
        let start = self.position;
        while self.position < self.chars.len() {
            let c = self.chars[self.position];
            if c.is_ascii_digit() || c == '.' {
                self.position += 1;
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.position].iter().collect();
        let n: f64 = s.parse().unwrap_or(0.0);
        Token::Number(n)
    }

    fn read_string(&mut self) -> Token {
        let quote = self.chars[self.position];
        self.position += 1;
        let mut s = String::new();
        while self.position < self.chars.len() {
            let c = self.chars[self.position];
            if c == quote {
                self.position += 1;
                return Token::String(s);
            }
            s.push(c);
            self.position += 1;
        }
        Token::String(s)
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.position;
        while self.position < self.chars.len() {
            let c = self.chars[self.position];
            if c.is_ascii_alphanumeric() || c == '_' || c == '$' {
                self.position += 1;
            } else {
                break;
            }
        }
        let name: String = self.chars[start..self.position].iter().collect();

        for (kw, keyword) in KEYWORDS {
            if name == *kw {
                return Token::Keyword(keyword.clone());
            }
        }
        Token::Identifier(name)
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.chars.len() {
            let c = self.chars[self.position];
            if c.is_ascii_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
    }
}

/// Recursive-descent parser.
struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn parse_program(mut self) -> Result<Vec<Statement>, JsError> {
        let mut statements = Vec::new();
        while self.peek() != Token::Eof {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, JsError> {
        match self.peek() {
            Token::Keyword(Keyword::Var)
            | Token::Keyword(Keyword::Let)
            | Token::Keyword(Keyword::Const) => {
                self.parse_variable_declaration()
            }
            Token::Keyword(Keyword::If) => {
                let stmt = self.parse_if()?;
                self.consume_optional_semicolon();
                Ok(stmt)
            }
            Token::Keyword(Keyword::While) => {
                let stmt = self.parse_while()?;
                self.consume_optional_semicolon();
                Ok(stmt)
            }
            Token::Keyword(Keyword::Return) => self.parse_return(),
            Token::Punctuator(Punctuator::LeftBrace) => {
                self.parse_block().map(Statement::Block)
            }
            _ => {
                let expr = self.parse_expression()?;
                if matches!(self.peek(), Token::Punctuator(Punctuator::Semicolon)) {
                    self.next();
                }
                Ok(Statement::ExpressionStatement(expr))
            }
        }
    }

    fn consume_optional_semicolon(&mut self) {
        if matches!(self.peek(), Token::Punctuator(Punctuator::Semicolon)) {
            self.next();
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<Statement, JsError> {
        self.next(); // consume var/let/const
        let name = match self.next() {
            Token::Identifier(name) => Ok(name),
            _ => Err(JsError::ExpectedIdentifier),
        }?;
        self.expect(Punctuator::Assign)?;
        let value = self.parse_expression()?;
        self.expect(Punctuator::Semicolon)?;
        Ok(Statement::VariableDeclaration { name, value })
    }

    fn parse_if(&mut self) -> Result<Statement, JsError> {
        self.next(); // if
        self.expect(Punctuator::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(Punctuator::RightParen)?;
        let consequent = Box::new(self.parse_statement()?);
        let alternate = if matches!(self.peek(), Token::Keyword(Keyword::Else)) {
            self.next(); // else
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        Ok(Statement::If {
            condition,
            consequent,
            alternate,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, JsError> {
        self.next(); // while
        self.expect(Punctuator::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(Punctuator::RightParen)?;
        let body = Box::new(self.parse_statement()?);
        Ok(Statement::While { condition, body })
    }

    fn parse_return(&mut self) -> Result<Statement, JsError> {
        self.next(); // return
        let value = self.parse_expression()?;
        self.expect(Punctuator::Semicolon)?;
        Ok(Statement::Return(value))
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, JsError> {
        self.expect(Punctuator::LeftBrace)?;
        let mut statements = Vec::new();
        while !matches!(self.peek(), Token::Punctuator(Punctuator::RightBrace) | Token::Eof) {
            statements.push(self.parse_statement()?);
        }
        self.expect(Punctuator::RightBrace)?;
        Ok(statements)
    }

    fn parse_expression(&mut self) -> Result<Expression, JsError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, JsError> {
        let expr = self.parse_logical_or()?;
        if matches!(self.peek(), Token::Punctuator(Punctuator::Assign)) {
            self.next();
            let value = self.parse_assignment()?;
            Ok(Expression::Assignment {
                target: Box::new(expr),
                value: Box::new(value),
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expression, JsError> {
        let mut left = self.parse_logical_and()?;
        while matches!(
            self.peek(),
            Token::Punctuator(Punctuator::OrPunct)
        ) {
            self.next();
            let right = self.parse_logical_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, JsError> {
        let mut left = self.parse_equality()?;
        while matches!(
            self.peek(),
            Token::Punctuator(Punctuator::AndPunct)
        ) {
            self.next();
            let right = self.parse_equality()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, JsError> {
        let mut left = self.parse_comparison()?;
        while matches!(
            self.peek(),
            Token::Punctuator(Punctuator::EqualEqual)
                | Token::Punctuator(Punctuator::BangEqual)
        ) {
            let op = match self.next() {
                Token::Punctuator(Punctuator::EqualEqual) => BinaryOp::Equal,
                Token::Punctuator(Punctuator::BangEqual) => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, JsError> {
        let mut left = self.parse_term()?;
        while matches!(
            self.peek(),
            Token::Punctuator(Punctuator::Less)
                | Token::Punctuator(Punctuator::LessEqual)
                | Token::Punctuator(Punctuator::Greater)
                | Token::Punctuator(Punctuator::GreaterEqual)
        ) {
            let op = match self.next() {
                Token::Punctuator(Punctuator::Less) => BinaryOp::LessThan,
                Token::Punctuator(Punctuator::LessEqual) => BinaryOp::LessEqual,
                Token::Punctuator(Punctuator::Greater) => BinaryOp::GreaterThan,
                Token::Punctuator(Punctuator::GreaterEqual) => BinaryOp::GreaterEqual,
                _ => unreachable!(),
            };
            let right = self.parse_term()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expression, JsError> {
        let mut left = self.parse_factor()?;
        while matches!(
            self.peek(),
            Token::Punctuator(Punctuator::Plus) | Token::Punctuator(Punctuator::Minus)
        ) {
            let op = match self.next() {
                Token::Punctuator(Punctuator::Plus) => BinaryOp::Plus,
                Token::Punctuator(Punctuator::Minus) => BinaryOp::Minus,
                _ => unreachable!(),
            };
            let right = self.parse_factor()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expression, JsError> {
        let mut left = self.parse_unary()?;
        while matches!(
            self.peek(),
            Token::Punctuator(Punctuator::Star) | Token::Punctuator(Punctuator::Slash)
        ) {
            let op = match self.next() {
                Token::Punctuator(Punctuator::Star) => BinaryOp::Multiply,
                Token::Punctuator(Punctuator::Slash) => BinaryOp::Divide,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, JsError> {
        match self.peek() {
            Token::Punctuator(Punctuator::Bang) => {
                self.next();
                let operand = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                })
            }
            Token::Punctuator(Punctuator::Minus) => {
                self.next();
                let operand = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Negate,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> Result<Expression, JsError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::Punctuator(Punctuator::LeftParen) => {
                    self.next();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Token::Punctuator(Punctuator::RightParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.peek(), Token::Punctuator(Punctuator::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Punctuator::RightParen)?;
                    expr = Expression::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                Token::Punctuator(Punctuator::Dot) => {
                    self.next();
                    match self.next() {
                        Token::Identifier(prop) => {
                            expr = Expression::Member {
                                object: Box::new(expr),
                                property: prop,
                            };
                        }
                        _ => return Err(JsError::ExpectedProperty),
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, JsError> {
        match self.next() {
            Token::Number(n) => Ok(Expression::Literal(JsValue::Number(n))),
            Token::String(s) => Ok(Expression::Literal(JsValue::String(s))),
            Token::Identifier(name) => Ok(Expression::Identifier(name)),
            Token::Keyword(Keyword::True) => Ok(Expression::Literal(JsValue::Boolean(true))),
            Token::Keyword(Keyword::False) => Ok(Expression::Literal(JsValue::Boolean(false))),
            Token::Keyword(Keyword::Null) => Ok(Expression::Literal(JsValue::Null)),
            Token::Keyword(Keyword::Undefined) => Ok(Expression::Literal(JsValue::Undefined)),
            Token::Punctuator(Punctuator::LeftParen) => {
                let expr = self.parse_expression()?;
                self.expect(Punctuator::RightParen)?;
                Ok(expr)
            }
            _ => Err(JsError::UnexpectedToken),
        }
    }

    fn expect(&mut self, p: Punctuator) -> Result<(), JsError> {
        match self.next() {
            Token::Punctuator(p2) if p2 == p => Ok(()),
            _ => Err(JsError::ExpectedPunctuator),
        }
    }

    fn peek(&self) -> Token {
        self.tokens.get(self.position).cloned().unwrap_or(Token::Eof)
    }

    fn next(&mut self) -> Token {
        let token = self.peek();
        self.position += 1;
        token
    }
}

/// Tree-walk interpreter.
struct Interpreter {
    globals: JsObject,
    env: Environment,
}

#[derive(Debug, Clone, Default)]
struct Environment {
    vars: Vec<(String, JsValue)>,
}

impl Environment {
    fn new() -> Self {
        Self { vars: Vec::new() }
    }

    fn get(&self, name: &str) -> Option<&JsValue> {
        self.vars.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    fn set(&mut self, name: String, value: JsValue) {
        if let Some(existing) = self.vars.iter_mut().find(|(k, _)| k == &name) {
            existing.1 = value;
        } else {
            self.vars.push((name, value));
        }
    }
}

impl Interpreter {
    fn execute_block(&mut self, statements: Vec<Statement>) -> Result<JsValue, JsError> {
        let mut last = JsValue::Undefined;
        for stmt in statements {
            last = self.execute_statement(stmt)?;
        }
        Ok(last)
    }

    fn execute_statement(&mut self, stmt: Statement) -> Result<JsValue, JsError> {
        match stmt {
            Statement::ExpressionStatement(expr) => self.evaluate_expression(expr),
            Statement::VariableDeclaration { name, value } => {
                let v = self.evaluate_expression(value)?;
                self.env.set(name, v.clone());
                Ok(JsValue::Undefined)
            }
            Statement::Block(stmts) => self.execute_block(stmts),
            Statement::If {
                condition,
                consequent,
                alternate,
            } => {
                let cond = self.evaluate_expression(condition)?;
                if self.is_truthy(&cond) {
                    self.execute_statement(*consequent)
                } else if let Some(alt) = alternate {
                    self.execute_statement(*alt)
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            Statement::While { condition, body } => {
                let mut last = JsValue::Undefined;
                loop {
                    let cond = self.evaluate_expression(condition.clone())?;
                    if !self.is_truthy(&cond) {
                        break;
                    }
                    last = self.execute_statement(*body.clone())?;
                }
                Ok(last)
            }
            Statement::Return(value) => self.evaluate_expression(value),
            Statement::Expression(_) => Ok(JsValue::Undefined),
        }
    }

    fn evaluate_expression(&mut self, expr: Expression) -> Result<JsValue, JsError> {
        match expr {
            Expression::Literal(v) => Ok(v),
            Expression::Identifier(name) => {
                if let Some(v) = self.env.get(&name) {
                    return Ok(v.clone());
                }
                if let Some(v) = self.globals.get(&name) {
                    return Ok(v.clone());
                }
                Ok(JsValue::Undefined)
            }
            Expression::Binary { left, op, right } => {
                let l = self.evaluate_expression(*left)?;
                let r = self.evaluate_expression(*right)?;
                self.apply_binary(op, l, r)
            }
            Expression::Unary { op, operand } => {
                let v = self.evaluate_expression(*operand)?;
                self.apply_unary(op, v)
            }
            Expression::Call { callee, args } => {
                let evaluated_args: Result<Vec<_>, _> =
                    args.into_iter().map(|a| self.evaluate_expression(a)).collect();
                let evaluated_args = evaluated_args?;
                let callee_val = self.evaluate_expression(*callee)?;
                match callee_val {
                    JsValue::Function(f) => self.call_function(f, evaluated_args),
                    _ => Err(JsError::NotAFunction),
                }
            }
            Expression::Member { object, property } => {
                let obj = self.evaluate_expression(*object)?;
                match obj {
                    JsValue::Object(o) => Ok(o
                        .get(&property)
                        .cloned()
                        .unwrap_or(JsValue::Undefined)),
                    _ => Ok(JsValue::Undefined),
                }
            }
            Expression::Assignment { target, value } => {
                let v = self.evaluate_expression(*value)?;
                match *target {
                    Expression::Identifier(name) => {
                        self.env.set(name.clone(), v.clone());
                        self.globals.set(name, v.clone());
                        Ok(v)
                    }
                    _ => Err(JsError::InvalidAssignmentTarget),
                }
            }
        }
    }

    fn apply_binary(&self, op: BinaryOp, left: JsValue, right: JsValue) -> Result<JsValue, JsError> {
        match op {
            BinaryOp::Plus => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a + b)),
                (JsValue::String(a), JsValue::String(b)) => {
                    Ok(JsValue::String(format!("{}{}", a, b)))
                }
                _ => Ok(JsValue::Undefined),
            },
            BinaryOp::Minus => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a - b)),
                _ => Ok(JsValue::Number(0.0)),
            },
            BinaryOp::Multiply => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a * b)),
                _ => Ok(JsValue::Number(0.0)),
            },
            BinaryOp::Divide => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a / b)),
                _ => Ok(JsValue::Number(0.0)),
            },
            BinaryOp::Equal => Ok(JsValue::Boolean(left == right)),
            BinaryOp::NotEqual => Ok(JsValue::Boolean(left != right)),
            BinaryOp::LessThan => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a < b)),
                _ => Ok(JsValue::Boolean(false)),
            },
            BinaryOp::LessEqual => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a <= b)),
                _ => Ok(JsValue::Boolean(false)),
            },
            BinaryOp::GreaterThan => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a > b)),
                _ => Ok(JsValue::Boolean(false)),
            },
            BinaryOp::GreaterEqual => match (&left, &right) {
                (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a >= b)),
                _ => Ok(JsValue::Boolean(false)),
            },
            BinaryOp::And => {
                if self.is_truthy(&left) {
                    Ok(right)
                } else {
                    Ok(left)
                }
            }
            BinaryOp::Or => {
                if self.is_truthy(&left) {
                    Ok(left)
                } else {
                    Ok(right)
                }
            }
        }
    }

    fn apply_unary(&self, op: UnaryOp, value: JsValue) -> Result<JsValue, JsError> {
        match op {
            UnaryOp::Not => Ok(JsValue::Boolean(!self.is_truthy(&value))),
            UnaryOp::Negate => match value {
                JsValue::Number(n) => Ok(JsValue::Number(-n)),
                _ => Ok(JsValue::Number(0.0)),
            },
        }
    }

    fn is_truthy(&self, value: &JsValue) -> bool {
        match value {
            JsValue::Undefined | JsValue::Null => false,
            JsValue::Boolean(b) => *b,
            JsValue::Number(n) => *n != 0.0,
            JsValue::String(s) => !s.is_empty(),
            JsValue::Object(_) => true,
            JsValue::Function(_) => true,
        }
    }

    fn call_function(&mut self, f: JsFunction, args: Vec<JsValue>) -> Result<JsValue, JsError> {
        // Simple: bind params and execute body.
        for (i, param) in f.params.iter().enumerate() {
            if i < args.len() {
                self.env.set(param.clone(), args[i].clone());
            }
        }
        self.execute_block(f.body)
    }
}

/// JavaScript engine errors.
#[derive(Debug, thiserror::Error)]
pub enum JsError {
    #[error("unexpected token")]
    UnexpectedToken,
    #[error("expected identifier")]
    ExpectedIdentifier,
    #[error("expected punctuator")]
    ExpectedPunctuator,
    #[error("expected property")]
    ExpectedProperty,
    #[error("not a function")]
    NotAFunction,
    #[error("invalid assignment target")]
    InvalidAssignmentTarget,
}

// Add the missing punctuator variants used by the parser.
impl Token {
    fn _or_punct() -> Punctuator {
        Punctuator::OrPunct
    }
    fn _and_punct() -> Punctuator {
        Punctuator::AndPunct
    }
}

/// Extra logical punctuators (used internally by the parser).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum LogicalPunct {
    Or,
    And,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_arithmetic() {
        let mut engine = JsEngine::new();
        let result = engine.execute("1 + 2 * 3;").unwrap();
        assert_eq!(result, JsValue::Number(7.0));
    }

    #[test]
    fn variable_declaration() {
        let mut engine = JsEngine::new();
        let _ = engine.execute("var x = 10; x + 5;").unwrap();
    }

    #[test]
    fn if_statement() {
        let mut engine = JsEngine::new();
        let result = engine.execute("if (true) { 1 } else { 2 };").unwrap();
        assert_eq!(result, JsValue::Number(1.0));
    }

    #[test]
    fn while_loop() {
        let mut engine = JsEngine::new();
        let result = engine.execute("var i = 0; while (i < 3) { i = i + 1; } i;").unwrap();
        assert_eq!(result, JsValue::Number(3.0));
    }
}
