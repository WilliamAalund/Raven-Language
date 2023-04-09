use crate::tokens::tokenizer::{Tokenizer, TokenizerState};
use crate::tokens::tokens::{Token, TokenTypes};
use crate::tokens::util::{parse_acceptable, parse_ident, parse_numbers};

pub fn next_code_token(tokenizer: &mut Tokenizer, bracket_depth: u64) -> Token {
    match tokenizer.last.token_type {
        _ => {
            if tokenizer.matches(";") {
                if (tokenizer.state.clone() as u64 & TokenizerState::CodeToStructTop as u64) != 0 {
                    tokenizer.state = TokenizerState::TopElementToStruct;
                }
                tokenizer.make_token(TokenTypes::LineEnd)
            } else if tokenizer.matches("{") {
                tokenizer.state += 0x1FF;
                tokenizer.make_token(TokenTypes::CodeStart)
            } else if tokenizer.matches("}") {
                if bracket_depth == 0 {
                    if (tokenizer.state.clone() as u64 & TokenizerState::CodeToStructTop as u64) != 0 {
                        tokenizer.state = TokenizerState::TopElementToStruct;
                    } else {
                        tokenizer.state = TokenizerState::TopElement;
                    }
                } else {
                    tokenizer.state -= 0x1FF;
                }
                tokenizer.make_token(TokenTypes::CodeEnd)
            } else if tokenizer.matches(",") {
                tokenizer.make_token(TokenTypes::ArgumentEnd)
            } else if tokenizer.matches("(") {
                tokenizer.make_token(TokenTypes::ParenOpen)
            } else if tokenizer.matches(")") {
                    tokenizer.make_token(TokenTypes::ParenClose)
            } else if tokenizer.matches(".") {
                if tokenizer.len == tokenizer.index {
                    return tokenizer.make_token(TokenTypes::EOF);
                }
                if (tokenizer.buffer[tokenizer.index] as char).is_numeric() {
                    tokenizer.index -= 1;
                    parse_numbers(tokenizer)
                } else {
                    parse_ident(tokenizer, TokenTypes::CallingType, &[b'(', b';', b'}'])
                }
            } else if tokenizer.matches("return") {
                tokenizer.make_token(TokenTypes::Return)
            } else if tokenizer.matches("break") {
                tokenizer.make_token(TokenTypes::Break)
            } else if tokenizer.matches("switch") {
                tokenizer.make_token(TokenTypes::Switch)
            } else if tokenizer.matches("for") {
                tokenizer.make_token(TokenTypes::For)
            } else if tokenizer.matches("while") {
                tokenizer.make_token(TokenTypes::While)
            } else if tokenizer.matches("if") {
                tokenizer.make_token(TokenTypes::If)
            } else if tokenizer.matches("else") {
                tokenizer.make_token(TokenTypes::Else)
            } else if tokenizer.matches("\"") {
                tokenizer.state = tokenizer.state & 0xFF;
                tokenizer.make_token(TokenTypes::StringStart)
            } else {
                let found = tokenizer.next_included()?;
                if (found as char).is_alphabetic() || found == b'_' {
                    parse_acceptable(tokenizer, TokenTypes::Variable)
                } else if found >= b'0' && found <= b'9' {
                    parse_numbers(tokenizer)
                } else {
                    tokenizer.make_token(TokenTypes::Operator)
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::tokens::util::check_types;

    use super::*;

    #[test]
    fn test_code() {
        let mut types = [TokenTypes::If, TokenTypes::ParenOpen, TokenTypes::Integer,
        TokenTypes::Operator, TokenTypes::Float, TokenTypes::ParenClose, TokenTypes::CallingType, TokenTypes::ParenOpen,
        TokenTypes::Variable, TokenTypes::ArgumentEnd, TokenTypes::Variable, TokenTypes::ParenClose, TokenTypes::CodeStart];
        let code = "if (1 + 2.2).function(arg, args) {\
        for testing in test {\
        while \"my_str\\\"continues!\"{\
        return something;\
        }\
        }\
        }";
        check_types(&types, code, TokenizerState::Code);
    }
}