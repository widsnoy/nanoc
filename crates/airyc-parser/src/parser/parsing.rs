use crate::{parser::Parser, syntax_kind::SyntaxKind};

impl Parser<'_> {
    pub(super) fn parse_root(&mut self) {
        self.start_node(SyntaxKind::COMP_UNIT);
        self.bump_trivia();

        loop {
            match self.peek() {
                SyntaxKind::CONST_KW => self.parse_const_decl(),
                SyntaxKind::EOF => break,
                _ => self.parse_decl_or_func_def(),
            }
            self.bump_trivia();
        }

        self.finish_node();
    }

    fn parse_const_decl(&mut self) {
        self.start_node(SyntaxKind::CONST_DECL);
        self.expect(SyntaxKind::CONST_KW);
        self.parse_type();
        self.parse_const_def();
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.parse_const_def();
        }

        self.expect(SyntaxKind::SEMI);
        self.finish_node();
    }

    fn parse_const_def(&mut self) {
        self.start_node(SyntaxKind::CONST_DEF);

        self.parse_pointers();

        self.parse_const_index_val();

        self.expect(SyntaxKind::EQ);
        self.parse_const_init_val();
        self.finish_node();
    }

    fn parse_decl_or_func_def(&mut self) {
        let cp_start = self.checkpoint();

        if self.at(SyntaxKind::VOID_KW) {
            self.start_node_at(cp_start, SyntaxKind::FUNC_DEF);
            self.start_node(SyntaxKind::FUNC_TYPE);
            self.bump(); // void
            self.parse_pointers();
            self.finish_node(); // FUNC_TYPE
            self.parse_name();
            self.parse_func_def_body();
            self.finish_node();
            return;
        }

        let cp_func_type = self.checkpoint();
        self.parse_type();

        let cp_vardef = self.checkpoint();

        self.parse_pointers();

        if self.at_1(SyntaxKind::L_PAREN) {
            self.start_node_at(cp_start, SyntaxKind::FUNC_DEF);

            self.start_node_at(cp_func_type, SyntaxKind::FUNC_TYPE);
            self.finish_node();

            self.parse_name();
            self.parse_func_def_body();
            self.finish_node();
        } else {
            self.start_node_at(cp_start, SyntaxKind::VAR_DECL);

            self.start_node_at(cp_vardef, SyntaxKind::VAR_DEF);

            self.parse_const_index_val();

            if self.at(SyntaxKind::EQ) {
                self.bump();
                self.parse_init_val();
            }

            self.finish_node();

            while self.at(SyntaxKind::COMMA) {
                self.bump();
                self.parse_var_def();
            }

            self.expect(SyntaxKind::SEMI);
            self.finish_node();
        }
    }

    fn parse_pointers(&mut self) {
        if !self.at(SyntaxKind::STAR) {
            return;
        }
        self.start_node(SyntaxKind::POINTER);
        while self.at(SyntaxKind::STAR) {
            self.bump();
            if self.at(SyntaxKind::CONST_KW) {
                self.bump();
            }
        }
        self.finish_node();
    }

    fn parse_func_def_body(&mut self) {
        self.expect(SyntaxKind::L_PAREN);
        if !self.at(SyntaxKind::R_PAREN) {
            self.parse_func_f_params();
        }
        self.expect(SyntaxKind::R_PAREN);
        self.parse_block();
    }

    fn parse_var_def(&mut self) {
        self.start_node(SyntaxKind::VAR_DEF);
        self.parse_pointers();
        self.parse_const_index_val();
        if self.at(SyntaxKind::EQ) {
            self.bump();
            self.parse_init_val();
        }
        self.finish_node();
    }

    fn parse_type(&mut self) {
        self.start_node(SyntaxKind::TYPE);
        let current_token = self.peek();
        if matches!(current_token, SyntaxKind::INT_KW | SyntaxKind::FLOAT_KW) {
            self.bump();
        } else if current_token == SyntaxKind::STRUCT_KW {
            self.bump();
            self.parse_name();
        } else {
            self.error("Expected type");
        }
        self.finish_node();
    }

    fn parse_const_init_val(&mut self) {
        self.parse_init_val_generic(
            SyntaxKind::CONST_INIT_VAL,
            Self::parse_const_init_val,
            Self::parse_const_exp,
        );
    }

    fn parse_init_val(&mut self) {
        self.parse_init_val_generic(SyntaxKind::INIT_VAL, Self::parse_init_val, Self::parse_exp);
    }

    fn parse_init_val_generic(
        &mut self,
        kind: SyntaxKind,
        parse_recursive: fn(&mut Self),
        parse_expr: fn(&mut Self),
    ) {
        self.start_node(kind);
        if self.at(SyntaxKind::L_BRACE) {
            self.bump();
            while !matches!(self.peek(), SyntaxKind::R_BRACE | SyntaxKind::EOF) {
                parse_recursive(self);
                if self.at(SyntaxKind::COMMA) {
                    self.bump();
                }
            }
            self.expect(SyntaxKind::R_BRACE);
        } else {
            parse_expr(self);
        }
        self.finish_node();
    }

    fn parse_func_f_params(&mut self) {
        self.start_node(SyntaxKind::FUNC_F_PARAMS);
        self.parse_func_f_param();
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.parse_func_f_param();
        }
        self.finish_node();
    }

    fn parse_func_f_param(&mut self) {
        self.start_node(SyntaxKind::FUNC_F_PARAM);
        self.parse_type();
        self.parse_pointers();
        self.parse_name();
        if self.at(SyntaxKind::L_BRACK) {
            self.bump();
            self.expect(SyntaxKind::R_BRACK);
            while self.at(SyntaxKind::L_BRACK) {
                self.bump();
                self.parse_const_exp();
                self.expect(SyntaxKind::R_BRACK);
            }
        }
        self.finish_node();
    }

    pub(super) fn parse_func_r_params(&mut self) {
        self.start_node(SyntaxKind::FUNC_R_PARAMS);
        self.parse_exp();
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.parse_exp();
        }
        self.finish_node();
    }

    pub(super) fn parse_block(&mut self) {
        self.start_node(SyntaxKind::BLOCK);
        self.expect(SyntaxKind::L_BRACE);
        while !matches!(self.peek(), SyntaxKind::R_BRACE | SyntaxKind::EOF) {
            self.parse_block_item();
        }
        self.expect(SyntaxKind::R_BRACE);
        self.finish_node();
    }

    fn parse_block_item(&mut self) {
        match self.peek() {
            SyntaxKind::INT_KW | SyntaxKind::FLOAT_KW | SyntaxKind::STRUCT_KW => {
                // 应该重构 parse_decl_or_func_def，但暂时重写
                self.start_node(SyntaxKind::VAR_DECL);
                self.parse_type();
                self.parse_var_def();
                while self.at(SyntaxKind::COMMA) {
                    self.bump();
                    self.parse_var_def();
                }
                self.expect(SyntaxKind::SEMI);
                self.finish_node();
            }
            SyntaxKind::CONST_KW => {
                self.parse_const_decl();
            }
            _ => {
                // 其他情况视为语句
                self.parse_statement();
            }
        }
    }

    pub(super) fn parse_name(&mut self) {
        self.start_node(SyntaxKind::NAME);
        self.expect(SyntaxKind::IDENT);
        self.finish_node();
    }

    pub(super) fn parse_const_index_val(&mut self) {
        self.start_node(SyntaxKind::CONST_INDEX_VAL);
        self.parse_name();
        while self.at(SyntaxKind::L_BRACK) {
            self.bump();
            self.parse_const_exp();
            self.expect(SyntaxKind::R_BRACK);
        }
        self.finish_node();
    }
}
