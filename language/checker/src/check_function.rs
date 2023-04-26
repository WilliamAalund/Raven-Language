use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use syntax::function::Function;
use syntax::{ParsingError, VariableManager};
use syntax::syntax::Syntax;
use syntax::types::Types;
use crate::check_code::verify_code;
use crate::output::TypesChecker;

pub async fn verify_function(process_manager: &TypesChecker, mut function: Arc<Function>, syntax: &Arc<Mutex<Syntax>>) -> Result<(), ParsingError> {
    let mut variable_manager = CheckerVariableManager { variables: HashMap::new() };

    verify_code(process_manager, &mut unsafe { Arc::get_mut_unchecked(&mut function) }.code, syntax, &mut variable_manager).await?;
    return Ok(());
}

#[derive(Clone)]
pub struct CheckerVariableManager {
    pub variables: HashMap<String, Types>
}

impl VariableManager for CheckerVariableManager {
    fn get_variable(&self, name: &String) -> Option<Types> {
        return self.variables.get(name).map(|found| found.clone());
    }
}