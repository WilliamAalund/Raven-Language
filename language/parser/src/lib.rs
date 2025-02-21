#![feature(try_trait_v2, get_mut_unchecked)]
#![feature(let_chains)]
extern crate core;

use crate::parser::top_parser::parse_top;
use crate::parser::util::ParserUtils;
use crate::tokens::tokenizer::Tokenizer;
use crate::tokens::tokens::TokenTypes;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use syntax::async_util::{HandleWrapper, NameResolver, UnparsedType};
use syntax::syntax::Syntax;

/// The Raven parser
pub mod parser;
/// The Raven tokenizer
pub mod tokens;

/// Parses a file into the syntax
pub async fn parse(syntax: Arc<Mutex<Syntax>>, handle: Arc<Mutex<HandleWrapper>>, name: String, file: String) {
    let mut tokenizer = Tokenizer::new(file.as_bytes());
    let mut tokens = Vec::default();
    loop {
        tokens.push(tokenizer.next());
        if tokens.last().unwrap().token_type == TokenTypes::EOF {
            break;
        }
    }

    let mut parser_utils = ParserUtils {
        buffer: file.as_bytes(),
        index: 0,
        tokens,
        syntax,
        file: name.clone(),
        imports: ImportNameResolver::new(name.clone()),
        handle,
    };

    parse_top(&mut parser_utils);
}

/// Basic name resolver implementation
#[derive(Clone)]
pub struct ImportNameResolver {
    /// The current file imports
    pub imports: Vec<String>,
    /// The current generics
    pub generics: HashMap<String, Vec<UnparsedType>>,
    /// The parent type
    pub parent: Option<String>,
    /// Last ID used on a code block label
    pub last_id: u32,
}

impl ImportNameResolver {
    /// Creates a new name resolver
    pub fn new(base: String) -> Self {
        return Self { imports: vec![base], generics: HashMap::default(), parent: None, last_id: 0 };
    }
}

impl NameResolver for ImportNameResolver {
    fn imports(&self) -> &Vec<String> {
        return &self.imports;
    }

    fn generic(&self, name: &String) -> Option<Vec<UnparsedType>> {
        return self.generics.get(name).cloned();
    }

    fn generics(&self) -> &HashMap<String, Vec<UnparsedType>> {
        return &self.generics;
    }

    fn boxed_clone(&self) -> Box<dyn NameResolver> {
        return Box::new(self.clone());
    }
}
