#[macro_export]
macro_rules! ast_node {
    ($Name:ident, $Kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $Name {
            syntax: SyntaxNode,
        }
        impl AstNode for $Name {
            fn cast(syntax: SyntaxNode) -> Option<Self> {
                if Self::can_cast(syntax.kind().into()) {
                    Some(Self { syntax })
                } else {
                    None
                }
            }
            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }

        impl $Name {
            pub fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$Kind
            }

            #[allow(unused)]
            pub fn child<N: AstNode>(&self) -> Option<N> {
                self.syntax.children().find_map(N::cast)
            }

            #[allow(unused)]
            pub fn children<N: AstNode>(&self) -> impl Iterator<Item = N> {
                self.syntax.children().filter_map(N::cast)
            }

            #[allow(unused)]
            pub fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
                self.syntax
                    .children_with_tokens()
                    .filter_map(|it| it.into_token())
                    .find(|it| it.kind() == kind.into())
            }
        }
    };
}

#[macro_export]
macro_rules! ast_enum {
    ($Name:ident { $($Variant:ident),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $Name {
            $($Variant($Variant),)*
        }

        impl $Name {
            pub fn can_cast(kind: SyntaxKind) -> bool {
                $($Variant::can_cast(kind) || )* false
            }
        }

        impl AstNode for $Name {
            fn cast(syntax: SyntaxNode) -> Option<Self> {
                let kind = syntax.kind().into();
                $(
                    if $Variant::can_cast(kind) {
                        return $Variant::cast(syntax).map($Name::$Variant);
                    }
                )*
                None
            }
            fn syntax(&self) -> &SyntaxNode {
                match self {
                    $($Name::$Variant(it) => &it.syntax,)*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! ast_methods {
    ($Name:ident {
        $($Method:ident( $($Arg:tt)* ) : $Ret:ident => $Op:ident),* $(,)?
    }) => {
        impl $Name {
            $(
                ast_methods!(@method $Method( $($Arg)* ) : $Ret => $Op);
            )*
        }
    };

    (@method $Method:ident() : $Ret:ident => child) => {
        pub fn $Method(&self) -> Option<$Ret> {
            self.child()
        }
    };

    (@method $Method:ident() : $Ret:ident => children) => {
        pub fn $Method(&self) -> impl Iterator<Item = $Ret> {
            self.children()
        }
    };

    (@method $Method:ident($Kind:ident) : SyntaxToken => token) => {
        pub fn $Method(&self) -> Option<SyntaxToken> {
            self.token(SyntaxKind::$Kind)
        }
    };

    (@method $Method:ident($Idx:literal) : $Ret:ident => nth) => {
        pub fn $Method(&self) -> Option<$Ret> {
            self.children().nth($Idx)
        }
    };

    (@method $Method:ident() : SyntaxToken => first_token) => {
        pub fn $Method(&self) -> Option<SyntaxToken> {
            self.syntax
                .children_with_tokens()
                .filter_map(|it| it.into_token())
                .find(|it| !SyntaxKind::from(it.kind()).is_trivia())
        }
    };
}
