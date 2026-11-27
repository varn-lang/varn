use super::flow::Flow;
use crate::checker::Checker;
use vn_core::ast::Program;
use vn_sem::bind::BindResult;

mod decl;
mod expr;
mod stmt;

impl<'r> Checker<'r> {
    pub(crate) fn check_definite_assignment(&mut self, program: &Program, bind: &BindResult) {
        let mut flow = Flow::default();
        self.da_block(&program.body, &mut flow, bind);
    }
}
