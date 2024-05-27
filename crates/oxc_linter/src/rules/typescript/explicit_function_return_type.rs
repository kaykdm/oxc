use std::collections::HashSet;

use oxc_ast::{
    ast::{
        ArrowFunctionExpression, BindingPatternKind, Expression, FunctionType, JSXAttributeItem,
        PropertyKind, Statement, TSType, TSTypeName,
    },
    AstKind,
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;

use crate::{
    ast_util::outermost_paren_parent,
    context::LintContext,
    rule::Rule,
    rules::eslint::array_callback_return::return_checker::{
        check_statement, StatementReturnStatus,
    },
    AstNode,
};

fn explicit_function_return_type_diagnostic(span0: Span) -> OxcDiagnostic {
    println!("span: {:?}\n", span0);
    OxcDiagnostic::warn("typescript-eslint(explicit-function-return-type):")
        .with_help("help")
        .with_labels([span0.into()])
}

#[derive(Debug, Default, Clone)]
pub struct ExplicitFunctionReturnType(Box<ExplicitFunctionReturnTypeConfig>);

#[derive(Debug, Default, Clone)]
pub struct ExplicitFunctionReturnTypeConfig {
    allow_expressions: bool,
    allow_typed_function_expressions: bool,
    allow_direct_const_assertion_in_arrow_functions: bool,
    allow_concise_arrow_function_expressions_starting_with_void: bool,
    allow_functions_without_type_parameters: bool,
    allowed_names: HashSet<String>,
    allow_higher_order_functions: bool,
    allow_iifes: bool,
}

impl std::ops::Deref for ExplicitFunctionReturnType {
    type Target = ExplicitFunctionReturnTypeConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

declare_oxc_lint!(
    /// ### What it does
    ///
    ///
    /// ### Why is this bad?
    ///
    ///
    /// ### Example
    /// ```javascript
    /// ```
    ExplicitFunctionReturnType,
    correctness
);

impl Rule for ExplicitFunctionReturnType {
    fn from_configuration(value: serde_json::Value) -> Self {
        let options: Option<&serde_json::Value> = value.get(0);
        Self(Box::new(ExplicitFunctionReturnTypeConfig {
            allow_expressions: options
                .and_then(|x| x.get("allowExpressions"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            allow_typed_function_expressions: options
                .and_then(|x| x.get("allowTypedFunctionExpressions"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            allow_direct_const_assertion_in_arrow_functions: options
                .and_then(|x| x.get("allowDirectConstAssertionInArrowFunctions"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            allow_concise_arrow_function_expressions_starting_with_void: options
                .and_then(|x| x.get("allowConciseArrowFunctionExpressionsStartingWithVoid"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            allow_functions_without_type_parameters: options
                .and_then(|x| x.get("allowFunctionsWithoutTypeParameters"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            allowed_names: options
                .and_then(|x| x.get("allowedNames"))
                .and_then(serde_json::Value::as_array)
                .map(|v| {
                    v.iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            allow_higher_order_functions: options
                .and_then(|x| x.get("allowHigherOrderFunctions"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            allow_iifes: options
                .and_then(|x| x.get("allowIIFEs"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        }))
    }
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        match node.kind() {
            AstKind::Function(func) => {
                if !func.is_declaration() & !func.is_expression() {
                    return;
                }

                if func.return_type.is_some() || is_constructor_or_setter(node, ctx) {
                    return;
                }
                if self.is_allowed_function(node, ctx) {
                    return;
                }
                if matches!(func.r#type, FunctionType::FunctionDeclaration) {
                    if self.allow_typed_function_expressions && func.return_type.is_some() {
                        return;
                    }
                    if self.does_immediately_return_function_expression(node) {
                        return;
                    }
                } else {
                    if self.allow_typed_function_expressions
                        && (self.is_valid_function_expression_return_type(node, ctx)
                            || self.ancestor_has_return_type(node, ctx))
                    {
                        println!();
                        return;
                    }

                    if self.does_immediately_return_function_expression(node) {
                        return;
                    }
                }

                // check_function_return_type()
                println!("Function report error -------------------------------------\n");
                println!("func: {:?}\n", func);
                println!("return_type: {:?}\n", func.return_type);
                if let Some(parent) = get_parent_node(node, ctx) {
                    // println!("parent: {:?}\n", parent);
                    match parent.kind() {
                        AstKind::MethodDefinition(def) => {
                            ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                                def.span.start,
                                def.value.params.span.start,
                            )));
                            return;
                        }
                        AstKind::PropertyDefinition(def) => {
                            ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                                def.span.start,
                                func.params.span.start,
                            )));

                            return;
                        }
                        AstKind::ObjectProperty(prop) => {
                            ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                                prop.span.start,
                                func.params.span.start,
                            )));

                            return;
                        }
                        _ => {}
                    }
                }
                if func.is_expression() {
                    ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                        func.span.start,
                        func.params.span.start,
                    )));
                } else if let Some(id) = &func.id {
                    ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                        func.span.start,
                        id.span.end,
                    )));
                } else {
                    ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                        func.span.start,
                        func.params.span.start,
                    )));
                }
            }
            AstKind::ArrowFunctionExpression(func) => {
                if func.return_type.is_some() {
                    return;
                }
                if self.check_arrow_function_with_void(func) {
                    return;
                }
                if self.is_allowed_function(node, ctx) {
                    return;
                }
                if self.allow_typed_function_expressions
                    && (self.is_valid_function_expression_return_type(node, ctx)
                        || self.ancestor_has_return_type(node, ctx))
                {
                    return;
                }
                if self.returns_const_assertion_directly(node) {
                    return;
                }

                if self.does_immediately_return_function_expression(node) {
                    return;
                }

                println!(
                    "ArrowFunctionExpression report error -------------------------------------\n"
                );
                println!("func: {:?}\n", func);
                println!("return_type: {:?}\n", func.return_type);
                if let Some(parent) = get_parent_node(node, ctx) {
                    // println!("parent: {:?}\n", parent);
                    match parent.kind() {
                        AstKind::MethodDefinition(def) => {
                            ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                                def.span.start,
                                def.value.params.span.start,
                            )));
                            return;
                        }
                        AstKind::PropertyDefinition(def) => {
                            ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                                def.span.start,
                                func.params.span.start,
                            )));

                            return;
                        }
                        AstKind::ObjectProperty(prop) => {
                            ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                                prop.span.start,
                                func.params.span.start,
                            )));

                            return;
                        }
                        _ => {}
                    }
                }
                ctx.diagnostic(explicit_function_return_type_diagnostic(Span::new(
                    func.params.span.end + 1,
                    func.params.span.end + 3,
                )));
            }
            _ => {}
        }
    }
}

impl ExplicitFunctionReturnType {
    fn check_arrow_function_with_void(&self, func: &ArrowFunctionExpression) -> bool {
        if !self.allow_concise_arrow_function_expressions_starting_with_void {
            return false;
        }
        if !func.expression {
            return false;
        }
        let Some(expr) = func.get_expression() else { return false };
        let Expression::UnaryExpression(unary_expr) = expr else { return false };
        return matches!(unary_expr.operator, UnaryOperator::Void);
    }
    fn is_allowed_function<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) -> bool {
        match node.kind() {
            AstKind::Function(func) => {
                if self.allow_functions_without_type_parameters && func.type_parameters.is_none() {
                    return true;
                }
                if self.allow_iifes && is_iife(node, ctx) {
                    return true;
                }
                if self.allowed_names.len() == 0 {
                    return false;
                }
                if let Some(id) = &func.id {
                    return self.allowed_names.contains(id.name.as_str());
                }
                return self.check_parent_for_is_allowed_function(node, ctx);
            }
            AstKind::ArrowFunctionExpression(func) => {
                if self.allow_functions_without_type_parameters && func.type_parameters.is_none() {
                    return true;
                }
                if self.allow_iifes && is_iife(node, ctx) {
                    return true;
                }
                if self.allowed_names.len() == 0 {
                    return false;
                }

                return self.check_parent_for_is_allowed_function(node, ctx);
            }
            _ => false,
        }
    }
    fn check_parent_for_is_allowed_function<'a>(
        &self,
        node: &AstNode<'a>,
        ctx: &LintContext<'a>,
    ) -> bool {
        let Some(parent) = get_parent_node(node, ctx) else { return false };
        match parent.kind() {
            AstKind::VariableDeclarator(decl) => {
                let BindingPatternKind::BindingIdentifier(id) = &decl.id.kind else {
                    return false;
                };

                return self.allowed_names.contains(id.name.as_str());
            }
            AstKind::MethodDefinition(def) => {
                let Some(name) = def.key.name() else { return false };
                return self.allowed_names.contains(name.as_str());
            }
            AstKind::PropertyDefinition(def) => {
                let Some(name) = def.key.name() else { return false };
                return self.allowed_names.contains(name.as_str());
            }
            AstKind::ObjectProperty(prop) => {
                let Some(name) = prop.key.name() else { return false };
                return self.allowed_names.contains(name.as_str());
            }
            _ => false,
        }
    }
    fn is_valid_function_expression_return_type<'a>(
        &self,
        node: &AstNode<'a>,
        ctx: &LintContext<'a>,
    ) -> bool {
        if self.check_typed_function_expression(node, ctx) {
            return true;
        }
        return self.check_allow_expressions(node, ctx);
    }

    fn check_typed_function_expression<'a>(
        &self,
        node: &AstNode<'a>,
        ctx: &LintContext<'a>,
    ) -> bool {
        let Some(parent) = get_parent_node(node, ctx) else { return false };
        return self.is_typed_parent(parent, Some(node))
            || self.is_property_of_object_with_type(parent, ctx)
            || is_constructor_argument(parent);
    }
    fn is_typed_parent(&self, parent: &AstNode, callee: Option<&AstNode>) -> bool {
        return is_type_assertion(parent)
            || is_variable_declarator_with_type_annotation(parent)
            || is_property_definition_with_type_annotation(parent)
            || is_function_argument(parent, callee)
            || is_typed_jsx(parent);
    }
    /**
     * Checks if a node is a property or a nested property of a typed object:
     * ```
     * const x: Foo = { prop: () => {} }
     * const x = { prop: () => {} } as Foo
     * const x = <Foo>{ prop: () => {} }
     * const x: Foo = { bar: { prop: () => {} } }
     * ```
     */
    fn is_property_of_object_with_type(&self, node: &AstNode, ctx: &LintContext) -> bool {
        if !matches!(node.kind(), AstKind::ObjectProperty(_)) {
            return false;
        }
        if !matches!(node.kind(), AstKind::ObjectProperty(_)) {
            return false;
        }
        let Some(parent) = ctx.nodes().parent_node(node.id()) else {
            return false;
        };
        if !matches!(parent.kind(), AstKind::ObjectExpression(_)) {
            return false;
        }
        let Some(obj_expr_parent) = get_parent_node(parent, ctx) else {
            return false;
        };
        return self.is_typed_parent(obj_expr_parent, None)
            || self.is_property_of_object_with_type(obj_expr_parent, ctx);
    }

    fn check_allow_expressions(&self, node: &AstNode, ctx: &LintContext) -> bool {
        let Some(parent) = ctx.nodes().parent_node(node.id()) else {
            return false;
        };
        return self.allow_expressions
            && !matches!(
                parent.kind(),
                AstKind::VariableDeclarator(_)
                    | AstKind::MethodDefinition(_)
                    | AstKind::ExportDefaultDeclaration(_)
                    | AstKind::PropertyDefinition(_)
            );
    }
    fn returns_const_assertion_directly(&self, node: &AstNode) -> bool {
        if !self.allow_direct_const_assertion_in_arrow_functions {
            return false;
        }
        let AstKind::ArrowFunctionExpression(func) = node.kind() else { return false };
        let Some(expr) = func.get_expression() else { return false };

        match expr {
            Expression::TSAsExpression(ts_expr) => {
                let TSType::TSTypeReference(ts_type) = &ts_expr.type_annotation else {
                    return false;
                };
                let TSTypeName::IdentifierReference(id_ref) = &ts_type.type_name else {
                    return false;
                };

                return id_ref.name == "const";
            }
            Expression::TSTypeAssertion(ts_expr) => {
                let TSType::TSTypeReference(ts_type) = &ts_expr.type_annotation else {
                    return false;
                };
                let TSTypeName::IdentifierReference(id_ref) = &ts_type.type_name else {
                    return false;
                };

                return id_ref.name == "const";
            }
            _ => false,
        }
    }

    fn ancestor_has_return_type<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) -> bool {
        let parent = match get_parent_node(node, ctx) {
            Some(parent) => parent,
            None => return false,
        };

        if let AstKind::ObjectProperty(prop) = parent.kind() {
            if let Expression::ArrowFunctionExpression(func) = &prop.value {
                if func.body.statements.is_empty() {
                    return false;
                }
                if func.return_type.is_some() {
                    return true;
                }
            }
        } else {
            if self.check_return_statement_and_bodyless(parent) {
                return false;
            }
        }

        for ancestor in ctx.nodes().ancestors(node.id()).skip(1) {
            match ctx.nodes().kind(ancestor) {
                AstKind::ArrowFunctionExpression(func) => {
                    if func.return_type.is_some() {
                        return true;
                    }
                }
                AstKind::Function(func) => {
                    if func.return_type.is_some() {
                        return true;
                    }
                }
                AstKind::VariableDeclarator(decl) => {
                    return decl.id.type_annotation.is_some();
                }
                AstKind::PropertyDefinition(def) => {
                    return def.type_annotation.is_some();
                }
                AstKind::ExpressionStatement(_) => {
                    let Some(expr_parent) = get_parent_node(ctx.nodes().get_node(ancestor), ctx)
                    else {
                        return false;
                    };
                    if !matches!(expr_parent.kind(), AstKind::FunctionBody(_)) {
                        return false;
                    }
                }
                _ => {}
            }
        }

        return false;
    }
    fn check_return_statement_and_bodyless(&self, node: &AstNode) -> bool {
        match node.kind() {
            AstKind::ReturnStatement(_) => true,
            AstKind::ArrowFunctionExpression(func) => {
                return func.body.statements.is_empty();
            }
            _ => false,
        }
    }

    /**
     * Checks if a function belongs to:
     * ```
     * () => () => ...
     * () => function () { ... }
     * () => { return () => ... }
     * () => { return function () { ... } }
     * function fn() { return () => ... }
     * function fn() { return function() { ... } }
     * ```
     */
    fn does_immediately_return_function_expression(&self, node: &AstNode) -> bool {
        if !self.allow_higher_order_functions {
            return false;
        }
        if let AstKind::ArrowFunctionExpression(arrow_func_expr) = node.kind() {
            if let Some(func_body_expr) = arrow_func_expr.get_expression() {
                return is_function(func_body_expr);
            };
        }
        all_return_statements_are_functions(node)
    }
}

// check function is IIFE (Immediately Invoked Function Expression)
fn is_iife<'a>(node: &AstNode<'a>, ctx: &LintContext<'a>) -> bool {
    let Some(parent) = get_parent_node(node, ctx) else {
        return false;
    };
    return matches!(parent.kind(), AstKind::CallExpression(_));
}
/**
 * Checks if a node belongs to:
 * ```
 * new Foo(() => {})
 *         ^^^^^^^^
 * ```
 */
fn is_constructor_argument(node: &AstNode) -> bool {
    return matches!(node.kind(), AstKind::NewExpression(_));
}

fn is_constructor_or_setter(node: &AstNode, ctx: &LintContext) -> bool {
    let Some(parent) = ctx.nodes().parent_node(node.id()) else {
        return false;
    };
    return is_constructor(parent) || is_setter(parent);
}

fn is_constructor(node: &AstNode) -> bool {
    let AstKind::MethodDefinition(method_def) = node.kind() else { return false };
    return method_def.kind.is_constructor();
}

fn is_setter(node: &AstNode) -> bool {
    match node.kind() {
        AstKind::MethodDefinition(method_def) => return method_def.kind.is_set(),
        AstKind::ObjectProperty(obj_prop) => {
            return matches!(obj_prop.kind, PropertyKind::Set);
        }
        _ => false,
    }
}

fn get_parent_node<'a, 'b>(
    node: &'b AstNode<'a>,
    ctx: &'b LintContext<'a>,
) -> Option<&'b AstNode<'a>> {
    let Some(parent) = outermost_paren_parent(node, ctx) else { return None };
    match parent.kind() {
        AstKind::Argument(_) => outermost_paren_parent(parent, ctx),
        _ => Some(parent),
    }
}

fn is_variable_declarator_with_type_annotation(node: &AstNode) -> bool {
    let AstKind::VariableDeclarator(var_decl) = node.kind() else { return false };

    return var_decl.id.type_annotation.is_some();
}

fn is_function_argument(parent: &AstNode, callee: Option<&AstNode>) -> bool {
    let AstKind::CallExpression(call_expr) = parent.kind() else { return false };

    if callee.is_none() {
        return true;
    }

    match call_expr.callee.without_parenthesized() {
        Expression::FunctionExpression(func_expr) => {
            let AstKind::Function(callee_func_expr) = callee.unwrap().kind() else { return false };
            return func_expr.span != callee_func_expr.span;
        }
        Expression::ArrowFunctionExpression(arrow_func_expr) => {
            let AstKind::ArrowFunctionExpression(callee_arrow_func_expr) = callee.unwrap().kind()
            else {
                return false;
            };
            return arrow_func_expr.span != callee_arrow_func_expr.span;
        }
        _ => true,
    }
}

fn is_type_assertion(parent: &AstNode) -> bool {
    return matches!(parent.kind(), AstKind::TSAsExpression(_) | AstKind::TSTypeAssertion(_));
}

/**
 * Checks if a node is a class property with a type annotation.
 * ```
 * public x: Foo = ...
 * ```
 */
fn is_property_definition_with_type_annotation(node: &AstNode) -> bool {
    let AstKind::PropertyDefinition(prop_def) = node.kind() else { return false };
    return prop_def.type_annotation.is_some();
}

/**
 * Checks if a node is type-constrained in JSX
 * ```
 * <Foo x={() => {}} />
 * <Bar>{() => {}}</Bar>
 * <Baz {...props} />
 * ```
 */
fn is_typed_jsx(node: &AstNode) -> bool {
    if matches!(node.kind(), AstKind::JSXExpressionContainer(_) | AstKind::JSXSpreadAttribute(_)) {
        return true;
    }

    let AstKind::JSXAttributeItem(jsx_attr_item) = node.kind() else { return false };
    return matches!(jsx_attr_item, JSXAttributeItem::SpreadAttribute(_));
}

fn is_function(expr: &Expression) -> bool {
    return matches!(
        expr,
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    );
}
fn all_return_statements_are_functions(node: &AstNode) -> bool {
    match node.kind() {
        AstKind::ArrowFunctionExpression(arrow_func_expr) => {
            check_return_statements(&arrow_func_expr.body.statements)
        }
        AstKind::Function(func) => {
            if let Some(func_body) = &func.body {
                check_return_statements(&func_body.statements)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn check_return_statements<'a>(statements: &'a [Statement<'a>]) -> bool {
    if statements.len() == 0 {
        return false;
    }

    let mut has_return = false;

    let all_statements_valid = statements.iter().all(|stmt| match stmt {
        Statement::ReturnStatement(return_stmt) => {
            if let Some(arg) = &return_stmt.argument {
                has_return = true;
                return is_function(arg);
            }
            return false;
        }
        _ => {
            let status = check_statement(stmt);
            if status == StatementReturnStatus::AlwaysExplicit {
                has_return = true;
            }
            return matches!(
                status,
                StatementReturnStatus::NotReturn | StatementReturnStatus::AlwaysExplicit
            );
        }
    });

    has_return && all_statements_valid
}

#[test]
fn test() {
    use crate::tester::Tester;
    use std::path::PathBuf;

    let pass = vec![
        ("return;", None, None, None),
        (
            "
        function test(): void {
          return;
        }
              ",
            None,
            None,
            None,
        ),
        (
            "
        var fn = function (): number {
          return 1;
        };
              ",
            None,
            None,
            None,
        ),
        (
            "
        var arrowFn = (): string => 'test';
              ",
            None,
            None,
            None,
        ),
        (
            "
        class Test {
          constructor() {}
          get prop(): number {
            return 1;
          }
          set prop() {}
          method(): void {
            return;
          }
          arrow = (): string => 'arrow';
        }
              ",
            None,
            None,
            None,
        ),
        (
            "fn(() => {});",
            Some(serde_json::json!([
              {
                "allowExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "fn(function () {});",
            Some(serde_json::json!([
              {
                "allowExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "[function () {}, () => {}];",
            Some(serde_json::json!([
              {
                "allowExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "(function () {});",
            Some(serde_json::json!([
              {
                "allowExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "(() => {})();",
            Some(serde_json::json!([
              {
                "allowExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "export default (): void => {};",
            Some(serde_json::json!([
              {
                "allowExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        var arrowFn: Foo = () => 'test';
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        var funcExpr: Foo = function () {
          return 'test';
        };
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "const x = (() => {}) as Foo;",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "const x = <Foo>(() => {});",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            Some(PathBuf::from("test.ts")),
        ),
        (
            "
        const x = {
          foo: () => {},
        } as Foo;
              ",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "
        const x = <Foo>{
          foo: () => {},
        };
              ",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            Some(PathBuf::from("test.ts")),
        ),
        (
            "
        const x: Foo = {
          foo: () => {},
        };
              ",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "
        const x = {
          foo: { bar: () => {} },
        } as Foo;
              ",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "
        const x = <Foo>{
          foo: { bar: () => {} },
        };
              ",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            Some(PathBuf::from("test.ts")),
        ),
        (
            "
        const x: Foo = {
          foo: { bar: () => {} },
        };
              ",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "
        type MethodType = () => void;

        class App {
          private method: MethodType = () => {};
        }
              ",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "const foo = <button onClick={() => {}} />;",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "const foo = <button on={{ click: () => {} }} />;",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "const foo = <Bar>{() => {}}</Bar>;",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "const foo = <Bar>{{ on: () => {} }}</Bar>;",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "const foo = <button {...{ onClick: () => {} }} />;",
            Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
            None,
            None,
        ),
        (
            "
        const myObj = {
          set myProp(val) {
            this.myProp = val;
          },
        };
              ",
            None,
            None,
            None,
        ),
        (
            "
        () => (): void => {};
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        () => function (): void {};
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        () => {
          return (): void => {};
        };
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        () => {
          return function (): void {};
        };
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        () => {
          const foo = 'foo';
          return function (): string {
            return foo;
          };
        };
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        function fn() {
          return (): void => {};
        }
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        function fn() {
          return function (): void {};
        }
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        function fn() {
          const bar = () => (): number => 1;
          return function (): void {};
        }
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        function fn(arg: boolean) {
          if (arg) {
            return () => (): number => 1;
          } else {
            return function (): string {
              return 'foo';
            };
          }

          return function (): void {};
        }
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        function FunctionDeclaration() {
          return function FunctionExpression_Within_FunctionDeclaration() {
            return function FunctionExpression_Within_FunctionExpression() {
              return () => {
                // ArrowFunctionExpression_Within_FunctionExpression
                return () =>
                  // ArrowFunctionExpression_Within_ArrowFunctionExpression
                  (): number =>
                    1; // ArrowFunctionExpression_Within_ArrowFunctionExpression_WithNoBody
              };
            };
          };
        }
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        () => () => {
          return (): void => {
            return;
          };
        };
              ",
            Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
            None,
            None,
        ),
        (
            "
        declare function foo(arg: () => void): void;
        foo(() => 1);
        foo(() => {});
        foo(() => null);
        foo(() => true);
        foo(() => '');
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        declare function foo(arg: () => void): void;
        foo?.(() => 1);
        foo?.bar(() => {});
        foo?.bar?.(() => null);
        foo.bar?.(() => true);
        foo?.(() => '');
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        class Accumulator {
          private count: number = 0;

          public accumulate(fn: () => number): void {
            this.count += fn();
          }
        }

        new Accumulator().accumulate(() => 1);
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        declare function foo(arg: { meth: () => number }): void;
        foo({
          meth() {
            return 1;
          },
        });
        foo({
          meth: function () {
            return 1;
          },
        });
        foo({
          meth: () => {
            return 1;
          },
        });
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        const func1 = (value: number) => ({ type: 'X', value }) as const;
        const func2 = (value: number) => ({ type: 'X', value }) as const;
        const func3 = (value: number) => x as const;
        const func4 = (value: number) => x as const;
              ",
            Some(serde_json::json!([
              {
                "allowDirectConstAssertionInArrowFunctions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        new Promise(resolve => {});
        new Foo(1, () => {});
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "const log = (message: string) => void console.log(message);",
            Some(
                serde_json::json!([{ "allowConciseArrowFunctionExpressionsStartingWithVoid": true }]),
            ),
            None,
            None,
        ),
        (
            "const log = (a: string) => a;",
            Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
            None,
            None,
        ),
        (
            "const log = <A,>(a: A): A => a;",
            Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
            None,
            None,
        ),
        (
            "
        function log<A>(a: A): A {
          return a;
        }
              ",
            Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
            None,
            None,
        ),
        (
            "
        function log(a: string) {
          return a;
        }
              ",
            Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
            None,
            None,
        ),
        (
            "
        const log = function <A>(a: A): A {
          return a;
        };
              ",
            Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
            None,
            None,
        ),
        (
            "
        const log = function (a: A): string {
          return a;
        };
              ",
            Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
            None,
            None,
        ),
        (
            "
        function test1() {
          return;
        }

        const foo = function test2() {
          return;
        };
              ",
            Some(serde_json::json!([
              {
                "allowedNames": ["test1", "test2"],
              },
            ])),
            None,
            None,
        ),
        (
            "
        const test1 = function () {
          return;
        };
        const foo = function () {
          return function test2() {};
        };
              ",
            Some(serde_json::json!([
              {
                "allowedNames": ["test1", "test2"],
              },
            ])),
            None,
            None,
        ),
        (
            "
        const test1 = () => {
          return;
        };
        export const foo = {
          test2() {
            return 0;
          },
        };
              ",
            Some(serde_json::json!([
              {
                "allowedNames": ["test1", "test2"],
              },
            ])),
            None,
            None,
        ),
        (
            "
        class Test {
          constructor() {}
          get prop() {
            return 1;
          }
          set prop() {}
          method() {
            return;
          }
          arrow = () => 'arrow';
          private method() {
            return;
          }
        }
              ",
            Some(serde_json::json!([
              {
                "allowedNames": ["prop", "method", "arrow"],
              },
            ])),
            None,
            None,
        ),
        (
            "
        const x = {
          arrowFn: () => {
            return;
          },
          fn: function () {
            return;
          },
        };
              ",
            Some(serde_json::json!([
              {
                "allowedNames": ["arrowFn", "fn"],
              },
            ])),
            None,
            None,
        ),
        (
            "
        type HigherOrderType = () => (arg1: string) => string;
        const x: HigherOrderType = () => arg1 => 'foo';
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
                "allowHigherOrderFunctions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
            type HigherOrderType = () => (arg1: string) => string;
            const x: HigherOrderType = () => arg1 => 'foo';
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
                "allowHigherOrderFunctions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        type HigherOrderType = () => (arg1: string) => (arg2: number) => string;
        const x: HigherOrderType = () => arg1 => arg2 => 'foo';
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
                "allowHigherOrderFunctions": false,
              },
            ])),
            None,
            None,
        ),
        (
            "
        interface Foo {
          foo: string;
          arrowFn: () => string;
        }

        function foo(): Foo {
          return {
            foo: 'foo',
            arrowFn: () => 'test',
          };
        }
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
                "allowHigherOrderFunctions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        type Foo = (arg1: string) => string;
        type Bar<T> = (arg2: string) => T;
        const x: Bar<Foo> = arg1 => arg2 => arg1 + arg2;
              ",
            Some(serde_json::json!([
              {
                "allowTypedFunctionExpressions": true,
                "allowHigherOrderFunctions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        let foo = function (): number {
          return 1;
        };
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        const foo = (function () {
          return 1;
        })();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        const foo = (() => {
          return 1;
        })();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        const foo = ((arg: number): number => {
          return arg;
        })(0);
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        const foo = (() => (() => 'foo')())();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        let foo = (() => (): string => {
          return 'foo';
        })()();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        let foo = (() => (): string => {
          return 'foo';
        })();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
                "allowHigherOrderFunctions": false,
              },
            ])),
            None,
            None,
        ),
        (
            "
        let foo = (() => (): string => {
          return 'foo';
        })()();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
                "allowHigherOrderFunctions": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        let foo = (() => (): void => {})()();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        let foo = (() => (() => {})())();
              ",
            Some(serde_json::json!([
              {
                "allowIIFEs": true,
              },
            ])),
            None,
            None,
        ),
        (
            "
        class Bar {
          bar: Foo = {
            foo: x => x + 1,
          };
        }
              ",
            None,
            None,
            None,
        ),
        (
            "
        class Bar {
          bar: Foo[] = [
            {
              foo: x => x + 1,
            },
          ];
        }
              ",
            None,
            None,
            None,
        ),
    ];

    let fail = vec![
        // (
        //     "
        // const x = {
        //   1: function () {
        //     reutrn;
        //   },
        // };
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowedNames": ["test", "1"],
        //       },
        //     ])),
        // ),
        // (
        //     "
        // let anyValue: any;
        // function foo(): any {
        //   anyValue = () => () => console.log('aa');
        // }
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": true,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // function fn(arg: boolean) {
        //     if (arg) return 'string';
        //     return function (): void {};
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        // ),
        // ------------------------------------------------------------
        // (
        //     "
        //     function test(a: number, b: number) {
        //       return;
        //     }
        //           ",
        //     None,
        //     None,
        //     None,
        // ),
        // (
        //     "
        // function test() {
        //   return;
        // }
        //       ",
        //     None,
        //     None,
        //     None,
        // ),
        // (
        //     "
        // var fn = function () {
        //   return 1;
        // };
        //       ",
        //     None,
        //     None,
        //     None,
        // ),
        // (
        //     "
        // var arrowFn = () => 'test';
        //       ",
        //     None,
        //     None,
        //     None,
        // ),
        // (
        //     "
        // class Test {
        //   constructor() {}
        //   get prop() {
        //     return 1;
        //   }
        //   set prop() {}
        //   method() {
        //     return;
        //   }
        //   arrow = () => 'arrow';
        //   private method() {
        //     return;
        //   }
        // }
        //       ",
        //     None,
        //     None,
        //     None,
        // ),
        // (
        //     "
        // function test() {
        //   return;
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowExpressions": true }])),
        //     None,
        //     None,
        // ),
        // (
        //     "const foo = () => {};",
        //     Some(serde_json::json!([{ "allowExpressions": true }])),
        //     None,
        //     None,
        // ),
        // (
        //     "const foo = function () {};",
        //     Some(serde_json::json!([{ "allowExpressions": true }])),
        //     None,
        //     None,
        // ),
        // (
        //     "export default () => {};",
        //     Some(serde_json::json!([{ "allowExpressions": true }])),
        //     None,
        //     None,
        // ),
        // (
        //     "export default function () {}",
        //     Some(serde_json::json!([{ "allowExpressions": true }])),
        //     None,
        //     None,
        // ),
        // (
        //     "
        // class Foo {
        //   public a = () => {};
        //   public b = function () {};
        //   public c = function test() {};

        //   static d = () => {};
        //   static e = function () {};
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowExpressions": true }])),
        //     None,
        //     None,
        // ),
        // (
        //     "var arrowFn = () => 'test';",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
        //     None,
        //     None,
        // ),
        // (
        //     "
        // function foo(): any {
        //   const bar = () => () => console.log('aa');
        // }
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": true,
        //       },
        //     ])),
        //     None,
        //     None,
        // ),
        // (
        //     "
        // let anyValue: any;
        // function foo(): any {
        //   anyValue = () => () => console.log('aa');
        // }
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": true,
        //       },
        //     ])),
        //     None,
        //     None,
        // ),
        // (
        //     "
        // class Foo {
        //   foo(): any {
        //     const bar = () => () => {
        //       return console.log('foo');
        //     };
        //   }
        // }
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": true,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // var funcExpr = function () {
        //   return 'test';
        // };
        //       ",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": true }])),
        // ),
        // (
        //     "const x = (() => {}) as Foo;",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        // (
        //     "
        // interface Foo {}
        // const x = {
        //   foo: () => {},
        // } as Foo;
        //       ",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        // (
        //     "
        // interface Foo {}
        // const x: Foo = {
        //   foo: () => {},
        // };
        //       ",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        // (
        //     "const foo = <button onClick={() => {}} />;",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        // (
        //     "const foo = <button on={{ click: () => {} }} />;",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        // (
        //     "const foo = <Bar>{() => {}}</Bar>;",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        // (
        //     "const foo = <Bar>{{ on: () => {} }}</Bar>;",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        // (
        //     "const foo = <button {...{ onClick: () => {} }} />;",
        //     Some(serde_json::json!([{ "allowTypedFunctionExpressions": false }])),
        // ),
        //   (
        //       "
        // function foo(): any {
        //   class Foo {
        //     foo = () => () => {
        //       return console.log('foo');
        //     };
        //   }
        // }
        //       ",
        //       Some(serde_json::json!([
        //         {
        //           "allowTypedFunctionExpressions": true,
        //         },
        //       ])),
        //   ),
        // ("() => () => {};", Some(serde_json::json!([{ "allowHigherOrderFunctions": true }]))),
        //   ("() => function () {};", Some(serde_json::json!([{ "allowHigherOrderFunctions": true }]))),
        // (
        //     "
        // () => {
        //   return () => {};
        // };
        //       ",
        //     Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        // ),
        //   (
        //       "
        // () => {
        //   return function () {};
        // };
        //       ",
        //       Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        //   ),
        //   (
        //       "
        // function fn() {
        //   return () => {};
        // }
        //       ",
        //       Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        //   ),
        // (
        //     "
        // function fn() {
        //   return function () {};
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        // ),
        // (
        //     "
        // function fn() {
        //   const bar = () => (): number => 1;
        //   const baz = () => () => 'baz';
        //   return function (): void {};
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        // ),
        // (
        //     "
        // function fn(arg: boolean) {
        //   if (arg) return 'string';
        //   return function (): void {};
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        // ),
        // (
        //     "
        // function FunctionDeclaration() {
        //   return function FunctionExpression_Within_FunctionDeclaration() {
        //     return function FunctionExpression_Within_FunctionExpression() {
        //       return () => {
        //         // ArrowFunctionExpression_Within_FunctionExpression
        //         return () =>
        //           // ArrowFunctionExpression_Within_ArrowFunctionExpression
        //           () =>
        //             1; // ArrowFunctionExpression_Within_ArrowFunctionExpression_WithNoBody
        //       };
        //     };
        //   };
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        // ),
        // (
        //     "
        // () => () => {
        //   return () => {
        //     return;
        //   };
        // };
        //       ",
        //     Some(serde_json::json!([{ "allowHigherOrderFunctions": true }])),
        // ),
        // (
        //     "
        // declare function foo(arg: () => void): void;
        // foo(() => 1);
        // foo(() => {});
        // foo(() => null);
        // foo(() => true);
        // foo(() => '');
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": false,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // class Accumulator {
        //   private count: number = 0;

        //   public accumulate(fn: () => number): void {
        //     this.count += fn();
        //   }
        // }

        // new Accumulator().accumulate(() => 1);
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": false,
        //       },
        //     ])),
        // ),
        // (
        //     "(() => true)();",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": false,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // declare function foo(arg: { meth: () => number }): void;
        // foo({
        //   meth() {
        //     return 1;
        //   },
        // });
        // foo({
        //   meth: function () {
        //     return 1;
        //   },
        // });
        // foo({
        //   meth: () => {
        //     return 1;
        //   },
        // });
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": false,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // type HigherOrderType = () => (arg1: string) => (arg2: number) => string;
        // const x: HigherOrderType = () => arg1 => arg2 => 'foo';
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": false,
        //         "allowHigherOrderFunctions": true,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // type HigherOrderType = () => (arg1: string) => (arg2: number) => string;
        // const x: HigherOrderType = () => arg1 => arg2 => 'foo';
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowTypedFunctionExpressions": false,
        //         "allowHigherOrderFunctions": false,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // const func = (value: number) => ({ type: 'X', value }) as any;
        // const func2 = (value: number) => ({ type: 'X', value }) as Action;
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowDirectConstAssertionInArrowFunctions": true,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // const func = (value: number) => ({ type: 'X', value }) as const;
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowDirectConstAssertionInArrowFunctions": false,
        //       },
        //     ])),
        // ),
        // (
        //     "const log = (message: string) => void console.log(message);",
        //     Some(serde_json::json!([
        //       { "allowConciseArrowFunctionExpressionsStartingWithVoid": false },
        //     ])),
        // ),
        // (
        //     "
        //         const log = (message: string) => {
        //           void console.log(message);
        //         };
        //       ",
        //     Some(
        //         serde_json::json!([{ "allowConciseArrowFunctionExpressionsStartingWithVoid": true }]),
        //     ),
        // ),
        // (
        //     "const log = <A,>(a: A) => a;",
        //     Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
        // ),
        //   (
        //       "
        // function log<A>(a: A) {
        //   return a;
        // }
        //       ",
        //       Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
        //   ),
        //   (
        //       "
        // const log = function <A>(a: A) {
        //   return a;
        // };
        //       ",
        //       Some(serde_json::json!([{ "allowFunctionsWithoutTypeParameters": true }])),
        //   ),
        // (
        //     "
        // function hoge() {
        //   return;
        // }
        // const foo = () => {
        //   return;
        // };
        // const baz = function () {
        //   return;
        // };
        // let [test, test2] = function () {
        //   return;
        // };
        // class X {
        //   [test] = function () {
        //     return;
        //   };
        // }
        // const x = {
        //   1: function () {
        //     reutrn;
        //   },
        // };
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowedNames": ["test", "1"],
        //       },
        //     ])),
        // ),
        // (
        //     "
        // const ignoredName = 'notIgnoredName';
        // class Foo {
        //   [ignoredName]() {}
        // }
        //       ",
        //     Some(serde_json::json!([{ "allowedNames": ["ignoredName"] }])),
        // ),
        //   (
        //       "
        // class Bar {
        //   bar = [
        //     {
        //       foo: x => x + 1,
        //     },
        //   ];
        // }
        //       ",
        //       None,
        //   ),
        // (
        //     "
        // const foo = (function () {
        //   return 'foo';
        // })();
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowIIFEs": false,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // const foo = (function () {
        //   return () => {
        //     return 1;
        //   };
        // })();
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowIIFEs": true,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // let foo = function () {
        //   return 'foo';
        // };
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowIIFEs": true,
        //       },
        //     ])),
        // ),
        // (
        //     "
        // let foo = (() => () => {})()();
        //       ",
        //     Some(serde_json::json!([
        //       {
        //         "allowIIFEs": true,
        //       },
        //     ])),
        // ),
    ];

    Tester::new(ExplicitFunctionReturnType::NAME, pass, fail).test_and_snapshot();
}
