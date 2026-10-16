//! Independent AST projection into the reference schema; no oracle-driven parser behavior.
use pprs::{ParseResult, ParseStatus, ast::*};
use serde_json::{Value, json};
fn node(kind: &str, fields: Value) -> Value {
    Value::Object(serde_json::Map::from_iter([
        (String::from("kind"), Value::String(kind.into())),
        (String::from("fields"), fields),
    ]))
}
fn bytes(value: &pprs::ByteString) -> Value {
    json!({"bytes":hex::encode(value.as_bytes())})
}
fn integer(value: i64) -> Value {
    json!({"integer":value.to_string()})
}
fn identifier(value: &Identifier, varlike: bool) -> Value {
    node(
        if varlike {
            "VarLikeIdentifier"
        } else {
            "Identifier"
        },
        json!({"name":bytes(value.bytes())}),
    )
}
fn name(value: &Name) -> Value {
    let mut raw = Vec::new();
    for (i, part) in value.parts().iter().enumerate() {
        if i > 0 {
            raw.push(b'\\');
        }
        raw.extend_from_slice(part.as_bytes());
    }
    node(
        match value.qualification() {
            Qualification::Normal => "Name",
            Qualification::FullyQualified => "Name_FullyQualified",
            Qualification::Relative => "Name_Relative",
        },
        json!({"name":{"bytes":hex::encode(raw)}}),
    )
}
fn target(value: &Target) -> Value {
    match value {
        Target::Name(value) => name(value),
        Target::Expr(value) => expr(value),
    }
}
fn member(value: &Member, varlike: bool) -> Value {
    match value {
        Member::Identifier(value) => identifier(value, varlike),
        Member::Expr(value) => expr(value),
    }
}
fn arg(value: &Argument) -> Value {
    if let Some(val) = &value.value {
        node(
            "Arg",
            json!({"name":value.name.as_ref().map(|v|identifier(v,false)),"value":expr(val),"byRef":value.by_ref,"unpack":value.unpack}),
        )
    } else {
        node("VariadicPlaceholder", json!({}))
    }
}
fn items(values: &[Option<ArrayItem>]) -> Value {
    json!(values.iter().map(|item|item.as_ref().map(|item|node("ArrayItem",json!({"key":item.key.as_ref().map(expr),"value":expr(&item.value),"byRef":item.by_ref,"unpack":item.unpack})))).collect::<Vec<_>>())
}
fn parts(values: &[StringPart]) -> Vec<Value> {
    values
        .iter()
        .map(|part| match part {
            StringPart::Text { value, .. } => {
                node("InterpolatedStringPart", json!({"value":bytes(value)}))
            }
            StringPart::Expr(value) => expr(value),
        })
        .collect()
}
fn ty(value: &Type) -> Value {
    match value {
        Type::Name(n) => name(n),
        Type::Builtin(n) => identifier(n, false),
        Type::Nullable { inner, .. } => node("NullableType", json!({"type":ty(inner)})),
        Type::Union { types, .. } => node(
            "UnionType",
            json!({"types":types.iter().map(ty).collect::<Vec<_>>()}),
        ),
        Type::Intersection { types, .. } => node(
            "IntersectionType",
            json!({"types":types.iter().map(ty).collect::<Vec<_>>()}),
        ),
    }
}
fn params(s: &Signature) -> Vec<Value> {
    s.params.iter().map(|p|node("Param",json!({"attrGroups":[],"flags":integer(0),"type":p.ty.as_ref().map(ty),"byRef":p.by_ref,"variadic":p.variadic,"var":expr(&p.variable),"default":p.default.as_ref().map(|v|expr(v)),"hooks":[]}))).collect()
}
fn binary(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "Plus",
        BinaryOp::Subtract => "Minus",
        BinaryOp::Multiply => "Mul",
        BinaryOp::Divide => "Div",
        BinaryOp::Concat => "Concat",
        BinaryOp::Modulo => "Mod",
        BinaryOp::Power => "Pow",
        BinaryOp::ShiftLeft => "ShiftLeft",
        BinaryOp::ShiftRight => "ShiftRight",
        BinaryOp::BitwiseAnd => "BitwiseAnd",
        BinaryOp::BitwiseOr => "BitwiseOr",
        BinaryOp::BitwiseXor => "BitwiseXor",
        BinaryOp::BooleanAnd => "BooleanAnd",
        BinaryOp::BooleanOr => "BooleanOr",
        BinaryOp::LogicalAnd => "LogicalAnd",
        BinaryOp::LogicalOr => "LogicalOr",
        BinaryOp::LogicalXor => "LogicalXor",
        BinaryOp::Equal => "Equal",
        BinaryOp::NotEqual => "NotEqual",
        BinaryOp::Identical => "Identical",
        BinaryOp::NotIdentical => "NotIdentical",
        BinaryOp::Less => "Smaller",
        BinaryOp::LessEqual => "SmallerOrEqual",
        BinaryOp::Greater => "Greater",
        BinaryOp::GreaterEqual => "GreaterOrEqual",
        BinaryOp::Spaceship => "Spaceship",
        BinaryOp::Coalesce => "Coalesce",
        _ => panic!("unmapped binary op"),
    }
}
fn unary(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Plus => "UnaryPlus",
        UnaryOp::Minus => "UnaryMinus",
        UnaryOp::Not => "BooleanNot",
        UnaryOp::BitwiseNot => "BitwiseNot",
        UnaryOp::Suppress => "ErrorSuppress",
        UnaryOp::PreIncrement => "PreInc",
        UnaryOp::PostIncrement => "PostInc",
        UnaryOp::PreDecrement => "PreDec",
        UnaryOp::PostDecrement => "PostDec",
        UnaryOp::Clone => "Clone",
        UnaryOp::Print => "Print",
        UnaryOp::Throw => "Throw",
        UnaryOp::YieldFrom => "YieldFrom",
        UnaryOp::Eval => "Eval",
        UnaryOp::Empty => "Empty",
    }
}
#[allow(
    clippy::too_many_lines,
    reason = "explicit reference projection for each public expression variant"
)]
pub fn expr(value: &Expr) -> Value {
    match &value.kind {
        ExprKind::Closure(c) => node(
            "Expr_Closure",
            json!({"attrGroups":[],"static":c.is_static,"byRef":c.signature.by_ref,"params":params(&c.signature),"uses":c.uses.iter().map(|u|node("ClosureUse",json!({"var":expr(&u.variable),"byRef":u.by_ref}))).collect::<Vec<_>>(),"returnType":c.signature.return_type.as_ref().map(ty),"stmts":statements(&c.body)}),
        ),
        ExprKind::ArrowFunction(c) => node(
            "Expr_ArrowFunction",
            json!({"attrGroups":[],"static":c.is_static,"byRef":c.signature.by_ref,"params":params(&c.signature),"returnType":c.signature.return_type.as_ref().map(ty),"expr":expr(&c.expr)}),
        ),
        ExprKind::Integer { value, .. } => node("Scalar_Int", json!({"value":integer(*value)})),
        ExprKind::Float(value) => node(
            "Scalar_Float",
            json!({"value":{"float64":format!("{:016x}",value.to_bits())}}),
        ),
        ExprKind::String { value, .. } => node("Scalar_String", json!({"value":bytes(value)})),
        ExprKind::Variable(value) => node("Expr_Variable", json!({"name":bytes(value.bytes())})),
        ExprKind::DynamicVariable(value) => node("Expr_Variable", json!({"name":expr(value)})),
        ExprKind::ConstFetch(value) => node("Expr_ConstFetch", json!({"name":name(value)})),
        ExprKind::MagicConstant(value) => node(
            &format!(
                "Scalar_MagicConst_{}",
                match value {
                    MagicConstant::Line => "Line",
                    MagicConstant::File => "File",
                    MagicConstant::Directory => "Dir",
                    MagicConstant::Class => "Class",
                    MagicConstant::Trait => "Trait",
                    MagicConstant::Method => "Method",
                    MagicConstant::Function => "Function",
                    MagicConstant::Namespace => "Namespace",
                    MagicConstant::Property => "Property",
                }
            ),
            json!({}),
        ),
        ExprKind::Assign { target, value } => node(
            "Expr_Assign",
            json!({"var":expr(target),"expr":expr(value)}),
        ),
        ExprKind::AssignRef { target, value } => node(
            "Expr_AssignRef",
            json!({"var":expr(target),"expr":expr(value)}),
        ),
        ExprKind::AssignOp { op, target, value } => node(
            &format!("Expr_AssignOp_{}", binary(*op)),
            json!({"var":expr(target),"expr":expr(value)}),
        ),
        ExprKind::Binary { op, left, right } => node(
            &format!("Expr_BinaryOp_{}", binary(*op)),
            json!({"left":expr(left),"right":expr(right)}),
        ),
        ExprKind::Unary { op, expr: value } => node(
            &format!("Expr_{}", unary(*op)),
            if matches!(
                op,
                UnaryOp::PreIncrement
                    | UnaryOp::PostIncrement
                    | UnaryOp::PreDecrement
                    | UnaryOp::PostDecrement
            ) {
                json!({"var":expr(value)})
            } else {
                json!({"expr":expr(value)})
            },
        ),
        ExprKind::Ternary {
            condition,
            then,
            otherwise,
        } => node(
            "Expr_Ternary",
            json!({"cond":expr(condition),"if":then.as_ref().map(|e|expr(e)),"else":expr(otherwise)}),
        ),
        ExprKind::Array { items: values, .. } => node("Expr_Array", json!({"items":items(values)})),
        ExprKind::List { items: values, .. } => node("Expr_List", json!({"items":items(values)})),
        ExprKind::ArrayDim { array, dim } => node(
            "Expr_ArrayDimFetch",
            json!({"var":expr(array),"dim":dim.as_ref().map(|e|expr(e))}),
        ),
        ExprKind::Call {
            target: value,
            args,
        } => node(
            "Expr_FuncCall",
            json!({"name":target(value),"args":args.iter().map(arg).collect::<Vec<_>>()}),
        ),
        ExprKind::Property {
            object,
            name,
            nullsafe,
        } => node(
            if *nullsafe {
                "Expr_NullsafePropertyFetch"
            } else {
                "Expr_PropertyFetch"
            },
            json!({"var":expr(object),"name":member(name,false)}),
        ),
        ExprKind::MethodCall {
            object,
            name,
            args,
            nullsafe,
        } => node(
            if *nullsafe {
                "Expr_NullsafeMethodCall"
            } else {
                "Expr_MethodCall"
            },
            json!({"var":expr(object),"name":member(name,false),"args":args.iter().map(arg).collect::<Vec<_>>()}),
        ),
        ExprKind::StaticProperty { class, name } => node(
            "Expr_StaticPropertyFetch",
            json!({"class":target(class),"name":member(name,true)}),
        ),
        ExprKind::ClassConst { class, name } => node(
            "Expr_ClassConstFetch",
            json!({"class":target(class),"name":member(name,false)}),
        ),
        ExprKind::StaticCall { class, name, args } => node(
            "Expr_StaticCall",
            json!({"class":target(class),"name":member(name,false),"args":args.iter().map(arg).collect::<Vec<_>>()}),
        ),
        ExprKind::New { class, args } => node(
            "Expr_New",
            json!({"class":target(class),"args":args.iter().map(arg).collect::<Vec<_>>()}),
        ),
        ExprKind::Instanceof { expr: value, class } => node(
            "Expr_Instanceof",
            json!({"expr":expr(value),"class":target(class)}),
        ),
        ExprKind::Cast { kind, expr: value } => node(
            &format!(
                "Expr_Cast_{}",
                match kind {
                    CastKind::Integer => "Int",
                    CastKind::Float => "Double",
                    CastKind::String => "String",
                    CastKind::Array => "Array",
                    CastKind::Object => "Object",
                    CastKind::Boolean => "Bool",
                    CastKind::Unset => "Unset",
                }
            ),
            json!({"expr":expr(value)}),
        ),
        ExprKind::Include { kind, expr: value } => node(
            "Expr_Include",
            json!({"expr":expr(value),"type":integer(match kind {IncludeKind::Include=>1,IncludeKind::IncludeOnce=>2,IncludeKind::Require=>3,IncludeKind::RequireOnce=>4})}),
        ),
        ExprKind::Isset(values) => node(
            "Expr_Isset",
            json!({"vars":values.iter().map(expr).collect::<Vec<_>>()}),
        ),
        ExprKind::Exit(value) => node("Expr_Exit", json!({"expr":value.as_ref().map(|e|expr(e))})),
        ExprKind::Yield { key, value } => node(
            "Expr_Yield",
            json!({"key":key.as_ref().map(|e|expr(e)),"value":value.as_ref().map(|e|expr(e))}),
        ),
        ExprKind::Match { condition, arms } => node(
            "Expr_Match",
            json!({"cond":expr(condition),"arms":arms.iter().map(|arm|node("MatchArm",json!({"conds":arm.conditions.as_ref().map(|c|c.iter().map(expr).collect::<Vec<_>>()),"body":expr(&arm.body)}))).collect::<Vec<_>>()}),
        ),
        ExprKind::Interpolated { parts: values, .. } => {
            node("Scalar_InterpolatedString", json!({"parts":parts(values)}))
        }
        ExprKind::ShellExec(values) => node("Expr_ShellExec", json!({"parts":parts(values)})),
        ExprKind::Error => node("Expr_Error", json!({})),
        _ => panic!("unmapped expression"),
    }
}
fn statements(stmts: &[Stmt]) -> Vec<Value> {
    let mut out = Vec::new();
    for stmt in stmts {
        out.push(match &stmt.kind {
            StmtKind::Expression(value) => node("Stmt_Expression", json!({"expr":expr(value)})),
            StmtKind::Nop => node("Stmt_Nop", json!({})),
            StmtKind::InlineHtml(value) => node("Stmt_InlineHTML", json!({"value":bytes(value)})),
            StmtKind::Echo(values) => node(
                "Stmt_Echo",
                json!({"exprs":values.iter().map(expr).collect::<Vec<_>>()}),
            ),
            StmtKind::Return(value) => {
                node("Stmt_Return", json!({"expr":value.as_ref().map(expr)}))
            }
            StmtKind::Block(values) => node("Stmt_Block", json!({"stmts":statements(values)})),
            StmtKind::HaltCompiler(value) => {
                node("Stmt_HaltCompiler", json!({"remaining":bytes(value)}))
            }
            StmtKind::Function(f) => node("Stmt_Function",json!({"attrGroups":[],"byRef":f.signature.by_ref,"name":identifier(&f.name,false),"params":params(&f.signature),"returnType":f.signature.return_type.as_ref().map(ty),"stmts":statements(&f.body)})),
            StmtKind::If(s) => node("Stmt_If",json!({"cond":expr(&s.condition),"stmts":statements(&s.body),"elseifs":s.elseifs.iter().map(|b|node("Stmt_ElseIf",json!({"cond":expr(&b.condition),"stmts":statements(&b.body)}))).collect::<Vec<_>>(),"else":s.otherwise.as_ref().map(|b|node("Stmt_Else",json!({"stmts":statements(&b.body)})))})),
            StmtKind::While(s) => node("Stmt_While",json!({"cond":expr(&s.condition),"stmts":statements(&s.body)})),
            StmtKind::Do(s) => node("Stmt_Do",json!({"cond":expr(&s.condition),"stmts":statements(&s.body)})),
            StmtKind::For(s) => node("Stmt_For",json!({"init":s.init.iter().map(expr).collect::<Vec<_>>(),"cond":s.conditions.iter().map(expr).collect::<Vec<_>>(),"loop":s.steps.iter().map(expr).collect::<Vec<_>>(),"stmts":statements(&s.body)})),
            StmtKind::Foreach(s) => node("Stmt_Foreach",json!({"expr":expr(&s.iterable),"keyVar":s.key.as_ref().map(|v|expr(v)),"valueVar":expr(&s.value),"byRef":s.by_ref,"stmts":statements(&s.body)})),
            StmtKind::Switch(s) => node("Stmt_Switch",json!({"cond":expr(&s.condition),"cases":s.cases.iter().map(|c|node("Stmt_Case",json!({"cond":c.condition.as_ref().map(|e|expr(e)),"stmts":statements(&c.body)}))).collect::<Vec<_>>()})),
            StmtKind::Break(e) => node("Stmt_Break",json!({"num":e.as_ref().map(expr)})),
            StmtKind::Continue(e) => node("Stmt_Continue",json!({"num":e.as_ref().map(expr)})),
            StmtKind::Goto(n) => node("Stmt_Goto",json!({"name":identifier(n,false)})),
            StmtKind::Label(n) => node("Stmt_Label",json!({"name":identifier(n,false)})),
            StmtKind::Global(v) => node("Stmt_Global",json!({"vars":v.iter().map(expr).collect::<Vec<_>>()})),
            StmtKind::Unset(v) => node("Stmt_Unset",json!({"vars":v.iter().map(expr).collect::<Vec<_>>()})),
            StmtKind::Static(v) => node("Stmt_Static",json!({"vars":v.iter().map(|v|node("StaticVar",json!({"var":expr(&v.variable),"default":v.default.as_ref().map(|e|expr(e))}))).collect::<Vec<_>>()})),
            StmtKind::Declare(s) => node("Stmt_Declare",json!({"declares":s.items.iter().map(|d|node("DeclareItem",json!({"key":identifier(&d.key,false),"value":expr(&d.value)}))).collect::<Vec<_>>(),"stmts":s.body.as_ref().map(|b|statements(b))})),
            StmtKind::Try(s) => node("Stmt_TryCatch",json!({"stmts":statements(&s.body),"catches":s.catches.iter().map(|c|node("Stmt_Catch",json!({"types":c.types.iter().map(name).collect::<Vec<_>>(),"var":c.variable.as_ref().map(|e|expr(e)),"stmts":statements(&c.body)}))).collect::<Vec<_>>(),"finally":s.finally.as_ref().map(|f|node("Stmt_Finally",json!({"stmts":statements(&f.body)})))})),
            _ => panic!("unmapped statement"),
        });
    }
    out
}
pub fn observation(result: &ParseResult, target: &str) -> Value {
    json!({"target":target,"status":match result.status(){ParseStatus::Clean=>"clean",ParseStatus::Recovered=>"recovered",ParseStatus::Aborted=>"aborted"},"ast":result.program().map(|p|statements(&p.statements))})
}
