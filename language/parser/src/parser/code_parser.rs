use std::future::Future;
use std::pin::Pin;
use syntax::code::{Effects, Expression, ExpressionType};
use syntax::function::CodeBody;
use syntax::ParsingError;
use syntax::types::Types;
use crate::parser::control_parser::parse_for;
use crate::parser::operator_parser::parse_operator;
use crate::parser::util::{add_generics, ParserUtils};
use crate::tokens::tokens::{Token, TokenTypes};

pub type ParsingFuture<T> = Pin<Box<dyn Future<Output=Result<T, ParsingError>> + Send>>;

pub fn parse_code(parser_utils: &mut ParserUtils) -> impl Future<Output=Result<CodeBody, ParsingError>> {
    let mut lines = Vec::new();
    while let Some((expression, effect)) = parse_line(parser_utils, false, false) {
        lines.push(get_line(effect, expression));
    }
    parser_utils.imports.last_id += 1;
    return create_body(parser_utils.imports.last_id - 1, lines);
}

pub fn parse_line(parser_utils: &mut ParserUtils, break_at_body: bool, deep: bool)
                  -> Option<(ExpressionType, ParsingFuture<Effects>)> {
    let mut effect = None;
    let mut expression_type = ExpressionType::Line;
    loop {
        let token = parser_utils.tokens.get(parser_utils.index).unwrap();
        parser_utils.index += 1;
        match token.token_type {
            TokenTypes::ParenOpen => {
                if let Some((_, in_effect)) = parse_line(parser_utils, break_at_body, true) {
                    effect = Some(in_effect);
                } else {
                    effect = None;
                }
            }
            TokenTypes::Float => {
                effect = Some(constant_effect(Effects::Float(token.to_string(parser_utils.buffer).parse().unwrap())))
            }
            TokenTypes::Integer => {
                effect = Some(constant_effect(Effects::Int(token.to_string(parser_utils.buffer).parse().unwrap())))
            }
            TokenTypes::LineEnd | TokenTypes::ParenClose => break,
            TokenTypes::CodeEnd | TokenTypes::BlockEnd => return None,
            TokenTypes::Variable => {
                effect = Some(constant_effect(Effects::LoadVariable(token.to_string(parser_utils.buffer))))
            }
            TokenTypes::Return => expression_type = ExpressionType::Return,
            TokenTypes::New => effect = Some(parse_new(parser_utils)),
            TokenTypes::BlockStart => if break_at_body {
                break;
            } else {
                effect = Some(Box::pin(body_effect(parse_code(parser_utils))))
            },
            TokenTypes::Let => return Some((expression_type, parse_let(parser_utils))),
            TokenTypes::For => return Some((expression_type, parse_for(parser_utils))),
            TokenTypes::Equals => if effect.is_some() &&
                parser_utils.tokens.get(parser_utils.index + 1).unwrap().token_type != TokenTypes::Operator {
                let error = token.make_error(parser_utils.file.clone(), "Tried to assign a void value!".to_string());
                let value = parse_line(parser_utils, false, false);
                if let Some(value) = value {
                    effect = Some(Box::pin(create_assign(effect.unwrap(), value.1)));
                } else {
                    effect = Some(constant_error(error));
                }
            } else {
                return Some((expression_type, parse_operator(effect, parser_utils)));
            },
            TokenTypes::Operator => return Some((expression_type, parse_operator(effect, parser_utils))),
            TokenTypes::ArgumentEnd => if !deep {
                break;
            },
            _ => panic!("How'd you get here? {:?}", token.token_type)
        }
    }
    return Some((expression_type, effect.unwrap_or(constant_effect(Effects::NOP()))));
}

async fn body_effect(body: impl Future<Output=Result<CodeBody, ParsingError>>) -> Result<Effects, ParsingError> {
    return Ok(Effects::CodeBody(body.await?));
}

fn constant_effect(effect: Effects) -> ParsingFuture<Effects> {
    return Box::pin(constant_effect_inner(Ok(effect)));
}

fn constant_error(error: ParsingError) -> ParsingFuture<Effects> {
    return Box::pin(constant_effect_inner(Err(error)));
}

async fn constant_effect_inner(effect: Result<Effects, ParsingError>) -> Result<Effects, ParsingError> {
    return effect;
}

async fn create_assign(last: ParsingFuture<Effects>, effect: ParsingFuture<Effects>) -> Result<Effects, ParsingError> {
    return Ok(Effects::Set(Box::new(last.await?), Box::new(effect.await?)));
}

fn parse_let(parser_utils: &mut ParserUtils) -> ParsingFuture<Effects> {
    let name;
    {
        let next = parser_utils.tokens.get(parser_utils.index).unwrap();
        if let TokenTypes::Variable = next.token_type {
            name = next.to_string(parser_utils.buffer);
        } else {
            return constant_error(next.make_error(parser_utils.file.clone(), "Unexpected token, expected variable name!".to_string()));
        }

        if let TokenTypes::Equals = next.token_type {} else {
            return constant_error(next.make_error(parser_utils.file.clone(), "Unexpected token, expected equals!".to_string()));
        }
    }

    return match parse_line(parser_utils, false, false) {
        Some(line) => Box::pin(create_let(name, line.1)),
        None => constant_error(parser_utils.tokens.get(parser_utils.index).unwrap()
            .make_error(parser_utils.file.clone(), "Expected value, found void!".to_string()))
    };
}

async fn create_let(name: String, value: ParsingFuture<Effects>) -> Result<Effects, ParsingError> {
    let value = value.await?;
    return Ok(Effects::CreateVariable(name, Box::new(value)));
}

fn parse_new(parser_utils: &mut ParserUtils) -> ParsingFuture<Effects> {
    let mut types: Option<ParsingFuture<Types>> = None;
    let values;

    loop {
        let token = parser_utils.tokens.get(parser_utils.index).unwrap();
        parser_utils.index += 1;
        match token.token_type {
            TokenTypes::Variable => {
                types = Some(Box::pin(parser_utils
                    .get_struct(token, token.to_string(parser_utils.buffer))))
            }
            //Handle making new structs with generics.
            TokenTypes::Operator => {
                types = Some(add_generics(types.unwrap(), parser_utils));
            }
            TokenTypes::BlockStart => {
                values = parse_new_args(parser_utils);
                break;
            }
            TokenTypes::InvalidCharacters => {}
            _ => panic!("How'd you get here? {:?}", token.token_type)
        }
    }

    return Box::pin(create_effect(Box::pin(types.unwrap()), values));
}

fn parse_new_args(parser_utils: &mut ParserUtils) -> Vec<(usize, ParsingFuture<Effects>)> {
    let mut values = Vec::new();
    let mut name = String::new();
    loop {
        let token = parser_utils.tokens.get(parser_utils.index).unwrap();
        parser_utils.index += 1;
        match token.token_type {
            TokenTypes::Variable => name = token.to_string(parser_utils.buffer),
            TokenTypes::Colon | TokenTypes::ArgumentEnd => {
                let effect = if let TokenTypes::Colon = token.token_type {
                    let token = token.clone();
                    parse_line(parser_utils, false, false).unwrap_or((ExpressionType::Line,
                                                                      Box::pin(expect_effect(parser_utils.file.clone(), token)))).1
                } else {
                    constant_effect(Effects::LoadVariable(name))
                };
                name = String::new();
                values.push((0, effect));
            }
            TokenTypes::BlockEnd => break,
            TokenTypes::InvalidCharacters => {}
            _ => panic!("How'd you get here? {:?}", token.token_type)
        }
    }

    return values;
}

async fn expect_effect(file: String, token: Token) -> Result<Effects, ParsingError> {
    return Err(token.make_error(file, "Expected something, found void".to_string()));
}

async fn create_effect(types: ParsingFuture<Types>, inputs: Vec<(usize, ParsingFuture<Effects>)>)
                       -> Result<Effects, ParsingError> {
    let mut final_inputs = Vec::new();
    for input in inputs {
        final_inputs.push((input.0, input.1.await?));
    }
    return Ok(Effects::CreateStruct(types.await?, final_inputs));
}

pub async fn get_line(effect: ParsingFuture<Effects>, expression_type: ExpressionType)
                      -> Result<Expression, ParsingError> {
    return Ok(Expression::new(expression_type, effect.await?));
}

pub async fn create_body(id: u32, lines: Vec<impl Future<Output=Result<Expression, ParsingError>>>)
                         -> Result<CodeBody, ParsingError> {
    let mut body = Vec::new();
    for line in lines {
        body.push(line.await?);
    }
    return Ok(CodeBody::new(body, id.to_string()));
}