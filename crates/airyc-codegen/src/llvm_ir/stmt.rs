use airyc_parser::ast::*;

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;

impl<'a, 'ctx> Program<'a, 'ctx> {
    /// 编译代码块
    pub(super) fn compile_block(&mut self, block: Block) -> Result<()> {
        self.symbols.push_scope();
        for item in block.items() {
            let is_terminal = if let BlockItem::Stmt(ref stmt) = item
                && matches!(
                    stmt,
                    Stmt::BreakStmt(_) | Stmt::ContinueStmt(_) | Stmt::ReturnStmt(_)
                ) {
                true
            } else {
                false
            };
            match item {
                BlockItem::VarDecl(decl) => self.compile_local_decl(decl)?,
                BlockItem::Stmt(stmt) => self.compile_stmt(stmt)?,
            }

            if is_terminal {
                break;
            }
        }
        self.symbols.pop_scope();
        Ok(())
    }

    pub(super) fn compile_stmt(&mut self, stmt: Stmt) -> Result<()> {
        match stmt {
            Stmt::AssignStmt(s) => self.compile_assign_stmt(s),
            Stmt::ExprStmt(s) => self.compile_expr_stmt(s),
            Stmt::Block(s) => self.compile_block(s),
            Stmt::IfStmt(s) => self.compile_if_stmt(s),
            Stmt::WhileStmt(s) => self.compile_while_stmt(s),
            Stmt::BreakStmt(s) => self.compile_break_stmt(s),
            Stmt::ContinueStmt(s) => self.compile_continue_stmt(s),
            Stmt::ReturnStmt(s) => self.compile_return_stmt(s),
        }
    }

    fn compile_assign_stmt(&mut self, stmt: AssignStmt) -> Result<()> {
        let rhs = self.compile_expr(stmt.rhs().ok_or(CodegenError::Missing("assign rhs"))?)?;
        let lhs_node = stmt.lhs().ok_or(CodegenError::Missing("assign lhs"))?;
        let lhs_ptr = self.get_expr_ptr(lhs_node)?;

        self.builder
            .build_store(lhs_ptr, rhs)
            .map_err(|_| CodegenError::LlvmBuild("assign store failed"))?;
        Ok(())
    }

    fn compile_expr_stmt(&mut self, stmt: ExprStmt) -> Result<()> {
        if let Some(expr) = stmt.expr() {
            self.compile_expr(expr)?;
        }
        Ok(())
    }

    fn compile_if_stmt(&mut self, stmt: IfStmt) -> Result<()> {
        let cond_val = self.compile_expr(
            stmt.condition()
                .ok_or(CodegenError::Missing("if condition"))?,
        )?;
        let func = self
            .symbols
            .current_function
            .ok_or(CodegenError::Missing("current function"))?;

        let then_bb = self.context.append_basic_block(func, "then");
        let else_bb = self.context.append_basic_block(func, "else");
        let merge_bb = self.context.append_basic_block(func, "merge");

        let bool_val = self.as_bool(cond_val)?;
        self.builder
            .build_conditional_branch(bool_val, then_bb, else_bb)
            .map_err(|_| CodegenError::LlvmBuild("if branch failed"))?;

        self.builder.position_at_end(then_bb);
        if let Some(t) = stmt.then_branch() {
            self.compile_stmt(t)?;
        }
        self.branch_if_no_terminator(merge_bb)?;

        self.builder.position_at_end(else_bb);
        if let Some(e) = stmt.else_branch() {
            self.compile_stmt(e)?;
        }
        self.branch_if_no_terminator(merge_bb)?;

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    fn compile_while_stmt(&mut self, stmt: WhileStmt) -> Result<()> {
        let func = self
            .symbols
            .current_function
            .ok_or(CodegenError::Missing("current function"))?;
        let cond_bb = self.context.append_basic_block(func, "while.cond");
        let body_bb = self.context.append_basic_block(func, "while.body");
        let end_bb = self.context.append_basic_block(func, "while.end");

        self.symbols.push_loop(cond_bb, end_bb);

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::LlvmBuild("while entry branch failed"))?;

        self.builder.position_at_end(cond_bb);
        let cond_val = self.compile_expr(
            stmt.condition()
                .ok_or(CodegenError::Missing("while condition"))?,
        )?;
        let bool_val = self.as_bool(cond_val)?;
        self.builder
            .build_conditional_branch(bool_val, body_bb, end_bb)
            .map_err(|_| CodegenError::LlvmBuild("while cond branch failed"))?;

        self.builder.position_at_end(body_bb);
        if let Some(body) = stmt.body() {
            self.compile_stmt(body)?;
        }
        self.branch_if_no_terminator(cond_bb)?;
        self.symbols.pop_loop();
        self.builder.position_at_end(end_bb);
        Ok(())
    }

    fn compile_break_stmt(&mut self, _stmt: BreakStmt) -> Result<()> {
        let end_bb = self
            .symbols
            .loop_stack
            .last()
            .ok_or(CodegenError::Unsupported("break not in loop".into()))?
            .end_bb;
        self.builder
            .build_unconditional_branch(end_bb)
            .map_err(|_| CodegenError::LlvmBuild("break branch failed"))?;
        Ok(())
    }

    fn compile_continue_stmt(&mut self, _stmt: ContinueStmt) -> Result<()> {
        let cond_bb = self
            .symbols
            .loop_stack
            .last()
            .ok_or(CodegenError::Unsupported("continue not in loop".into()))?
            .cond_bb;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|_| CodegenError::LlvmBuild("continue branch failed"))?;
        Ok(())
    }

    fn compile_return_stmt(&mut self, stmt: ReturnStmt) -> Result<()> {
        if let Some(expr) = stmt.expr() {
            let val = self.compile_expr(expr)?;
            self.builder.build_return(Some(&val)).ok();
        } else {
            self.builder.build_return(None).ok();
        }
        Ok(())
    }
}
