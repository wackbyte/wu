use super::*;
use std::fmt::*;

#[derive(Debug, Clone)]
pub struct Codegen<'c> {
    pub ast: &'c Vec<Statement>,
}

impl<'c> Codegen<'c> {
    pub fn new(ast: &'c Vec<Statement>)-> Self {
        Codegen {
            ast,
        }
    }

    pub fn generate(&self) -> String {
        let mut code = String::new();

        for statement in self.ast.iter() {
            code.push_str(&format!("{}\n", self.gen_statement(&statement.0)))
        }
        
        code
    }

    fn gen_statement(&self, statement: &StatementNode) -> String {
        use StatementNode::*;

        match *statement {
            Expression(ref expression) => format!("{}\n", self.gen_expression(&expression.0)),
            Return(ref value)          => format!("return{}\n", match *value {
                Some(ref v) => format!(" {}", self.gen_expression(&v.0)),
                None        => String::from("\n"),
            }),
            Definition { ref left, ref right, .. } => match *right {
                Some(ref right) => match right.0 {
                    ref block @ ExpressionNode::Block(_) => {
                        format!("local {}\n{}\n", self.gen_expression(&left.0), self.gen_block_assignment(&block, &left.0))
                        
                    },
                    _ => format!("local {} = {}\n", self.gen_expression(&left.0), self.gen_expression(&right.0)) 
                }
                None => format!("local {}\n", self.gen_expression(&left.0)),
            },

            ConstDefinition { ref left, ref right, .. } => match right.0 {
                ref block @ ExpressionNode::Block(_) => {
                    format!("local {}\n{}\n", self.gen_expression(&left.0), self.gen_block_assignment(&block, &left.0))
                },
                _ => format!("local {} = {}\n", self.gen_expression(&left.0), self.gen_expression(&right.0)) 
            },

            Assignment { ref left, ref right, .. } => format!("{} = {}", self.gen_expression(&left.0), self.gen_expression(&right.0)),
            If(ref if_node) => self.gen_if_node(if_node),
        }
    }

    fn gen_if_node(&self, if_node: &IfNode) -> String {
        let mut code = format!("if {} then\n{}", self.gen_expression(&if_node.condition.0), self.gen_expression(&if_node.body.0));

        if let Some(ref cases) = if_node.elses {
            for case in cases {
                let case_code = match *case {
                    (Some(ref condition), ref body, _) => format!("elseif {} then\n{}", self.gen_expression(&condition.0), self.gen_expression(&body.0)),
                    (None,                ref body, _) => format!("else\n{}", self.gen_expression(&body.0)),
                };

                code.push_str(&case_code)
            }

            code.push_str("end\n")
        } else {
            code.push_str("end\n");
        }

        code
    }

    fn gen_if_node_return(&self, if_node: &IfNode) -> String {
        let mut code = format!("if {} then\n{}", self.gen_expression(&if_node.condition.0), self.gen_block_return(&if_node.body.0));
        
        if let Some(ref cases) = if_node.elses {
            for case in cases {
                let case_code = match *case {
                    (Some(ref condition), ref body, _) => format!("elseif {} then\n{}\n", self.gen_expression(&condition.0), self.gen_block_return(&body.0)),
                    (None,                ref body, _) => format!("else\n{}\n", self.gen_block_return(&body.0)),
                };

                code.push_str(&case_code)
            }

            code.push_str("end\n")
        } else {
            code.push_str("end\n");
        }

        code
    }
    
    fn gen_if_node_assignment(&self, if_node: &IfNode, left: &ExpressionNode) -> String {
        let mut code = format!("if {} then\n{}", self.gen_expression(&if_node.condition.0), self.gen_block_assignment(&if_node.body.0, left));
        
        if let Some(ref cases) = if_node.elses {
            for case in cases {
                let case_code = match *case {
                    (Some(ref condition), ref body, _) => format!("elseif {} then\n{}\n", self.gen_expression(&condition.0), self.gen_block_assignment(&body.0, left)),
                    (None,                ref body, _) => format!("else\n{}\n", self.gen_block_assignment(&body.0, left)),
                };

                code.push_str(&case_code)
            }

            code.push_str("end\n")
        } else {
            code.push_str("end\n");
        }

        code
    }
    
    fn gen_statement_return(&self, statement: &StatementNode) -> String {
        use StatementNode::*;

        match *statement {
            If(ref if_node) => self.gen_if_node_return(if_node),
            Return(_)       => format!("{}\n", self.gen_statement(statement)),
            _               => format!("return {}\n", self.gen_statement(statement)),
        }
    }

    fn gen_statement_assignment(&self, statement: &StatementNode, left: &ExpressionNode) -> String {
        use StatementNode::*;

        match *statement {
            If(ref if_node) => self.gen_if_node_assignment(if_node, left),
            Return(_)       => self.gen_statement(statement),
            _               => format!("{} = {}\n", self.gen_expression(left), self.gen_statement(statement)),
        }
    }

    fn gen_expression(&self, expression: &ExpressionNode) -> String {
        use ExpressionNode::*;

        match *expression {
            Float(ref n)      => format!("{}", n),
            Int(ref n)        => format!("{}", n),
            Str(ref n)        => format!("{:?}", n),
            Bool(ref n)       => format!("{}", n),
            Identifier(ref n) => format!("{}", n),

            Call(ref callee, ref args) => {
                let mut code = self.gen_expression(&callee.0);

                code.push('(');

                let mut acc = 1;

                for arg in args.iter() {
                    code.push_str(&self.gen_expression(&arg.0));

                    if acc != args.len() {
                        code.push(',');
                    }

                    acc += 1
                }

                code.push(')');

                code
            }

            Binary { ref left, ref op, ref right } => self.gen_operation(&left.0, op, &right.0),

            Block(ref statements) => {
                if statements.len() == 1 {
                    format!("{}", self.gen_statement(&statements.last().unwrap().0))
                } else {
                    let mut code = "do\n".to_string();

                    for statement in statements {
                        code.push_str(&self.gen_statement(&statement.0))
                    }

                    code.push_str("end\n");

                    code
                }
            }

            Function { ref params, ref body, .. } => {
                let mut code = "function(".to_string();

                let mut acc = 1;

                let mut guards = Vec::new();

                for param in params {
                    if let Some(ref value) = param.2 {
                        guards.push((param.0.clone(), value.clone()))
                    }
                    
                    code.push_str(&param.0);

                    if acc != params.len() {
                        code.push(',');
                    }

                    acc += 1
                }

                code.push_str(")\n");

                for guard in guards {
                    code.push_str(&format!("if {0} == nil then\n{0} = {1}\nend\n", guard.0, self.gen_expression(&(guard.1).0)))
                }

                match body.0 {
                    Block(_) => code.push_str(&self.gen_block_return(&body.0)),
                    _        => code.push_str(&format!("return {}\n", self.gen_expression(&body.0))),
                }

                code.push_str("end\n");

                code
            }

            _ => String::new(),
        }
    }

    fn gen_operation(&self, left: &ExpressionNode, op: &Operator, right: &ExpressionNode) -> String {
        use Operator::*;
        use ExpressionNode::*;
        
        let c = match *op {
            PipeRight => {
                let compiled_left  = self.gen_expression(left);
                let compiled_right = self.gen_expression(right);

                format!("{}({})", compiled_right, compiled_left)
            },
            
            PipeLeft => {
                let compiled_left  = self.gen_expression(left);
                let compiled_right = self.gen_expression(right);

                format!("{}({})", compiled_left, compiled_right)
            },
            
            _ => {
                let compiled_left  = self.gen_expression(left);
                let compiled_op    = self.gen_operator(op);
                let compiled_right = self.gen_expression(right);

                match *right {
                    Int(_)        |
                    Float(_)      |
                    Str(_)        |
                    Bool(_)       |
                    Identifier(_) => format!("{}{}{}", compiled_left, compiled_op, compiled_right),
                    _             => format!("{}{}({})", compiled_left, compiled_op, compiled_right),
                }
            }
        };

        c
    }
    
    fn gen_operator(&self, op: &Operator) -> String {
        use Operator::*;
        
        let c = match *op {
            Add     => "+",
            Sub     => "-",
            Mul     => "*",
            Div     => "/",
            Mod     => "%",
            Pow     => "^",
            Equal   => "==",
            NEqual  => "~=",
            Lt      => "<",
            LtEqual => "<=",
            Gt      => ">",
            GtEqual => ">=",
            Concat  => "..",
            _       => "",
        };

        c.to_owned()
    }

    fn gen_block_assignment(&self, block: &ExpressionNode, left: &ExpressionNode) -> String {
        use ExpressionNode::*;

        if let Block(ref statements) = *block {
            if statements.len() == 1 {
                self.gen_statement_assignment(&statements.last().unwrap().0, left)
            } else {
                let mut code = "do\n".to_string();

                let mut acc = 1;

                for statement in statements {
                    if acc == statements.len() {
                        code.push_str(&self.gen_statement_assignment(&statement.0, left))
                    } else {
                        code.push_str(&format!("{}\n", self.gen_statement(&statement.0)));
                    }

                    acc += 1
                }

                code.push_str("end\n");

                code
            }
        } else {
            String::new()
        }
    }
    
    fn gen_block_return(&self, block: &ExpressionNode) -> String {
        use ExpressionNode::*;

        if let Block(ref statements) = *block {
            if statements.len() == 1 {
                self.gen_statement_return(&statements.last().unwrap().0)
            } else {
                let mut code = "do\n".to_string();

                let mut acc = 1;

                for statement in statements {
                    if acc == statements.len() {
                        code.push_str(&self.gen_statement_return(&statement.0))
                    } else {
                        code.push_str(&format!("{}\n", self.gen_statement_return(&statement.0)));
                    }

                    acc += 1
                }

                code.push_str("end\n");

                code
            }
        } else {
            String::new()
        }
    }
}

impl<'c> Display for Codegen<'c> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.generate())
    }
}