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
    mnode(
        value.meta(),
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
    mnode(
        value.meta(),
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
        Target::Expr(value) => expression(value),
    }
}
fn member(value: &Member, varlike: bool) -> Value {
    match value {
        Member::Identifier(value) => identifier(value, varlike),
        Member::Expr(value) => expression(value),
    }
}
fn arg(value: &Argument) -> Value {
    if let Some(val) = &value.value {
        mnode(
            &value.meta,
            "Arg",
            json!({"name":value.name.as_ref().map(|v|identifier(v,false)),"value":expression(val),"byRef":value.by_ref,"unpack":value.unpack}),
        )
    } else {
        mnode(&value.meta, "VariadicPlaceholder", json!({}))
    }
}
fn items(values: &[Option<ArrayItem>]) -> Value {
    json!(values.iter().map(|item|item.as_ref().map(|item|mnode(&item.meta,"ArrayItem",json!({"key":item.key.as_ref().map(expression),"value":expression(&item.value),"byRef":item.by_ref,"unpack":item.unpack})))).collect::<Vec<_>>())
}
fn parts(values: &[StringPart]) -> Vec<Value> {
    values
        .iter()
        .map(|part| match part {
            StringPart::Text { value, meta } => mnode(
                meta,
                "InterpolatedStringPart",
                json!({"value":bytes(value)}),
            ),
            StringPart::Expr(value) => expression(value),
        })
        .collect()
}
fn ty(value: &Type) -> Value {
    match value {
        Type::Name(n) => name(n),
        Type::Builtin(n) => identifier(n, false),
        Type::Nullable { inner, meta } => mnode(meta, "NullableType", json!({"type":ty(inner)})),
        Type::Union { types, meta } => mnode(
            meta,
            "UnionType",
            json!({"types":types.iter().map(ty).collect::<Vec<_>>()}),
        ),
        Type::Intersection { types, meta } => mnode(
            meta,
            "IntersectionType",
            json!({"types":types.iter().map(ty).collect::<Vec<_>>()}),
        ),
    }
}
fn params(s: &Signature) -> Vec<Value> {
    parameters(&s.params)
}
fn parameters(values: &[Parameter]) -> Vec<Value> {
    values.iter().map(|p|mnode(&p.meta,"Param",json!({"attrGroups":attributes(&p.attributes),"flags":integer(i64::from(p.flags.0)),"type":p.ty.as_ref().map(ty),"byRef":p.by_ref,"variadic":p.variadic,"var":expression(&p.variable),"default":p.default.as_ref().map(|v|expression(v)),"hooks":p.hooks.iter().map(hook).collect::<Vec<_>>()}))).collect()
}
fn attributes(values: &[AttributeGroup]) -> Vec<Value> {
    values.iter().map(|g|mnode(&g.meta,"AttributeGroup",json!({"attrs":g.attributes.iter().map(|a|mnode(&a.meta,"Attribute",json!({"name":name(&a.name),"args":a.args.iter().map(arg).collect::<Vec<_>>()}))).collect::<Vec<_>>()}))).collect()
}
fn hook(h: &PropertyHook) -> Value {
    let body = match &h.body {
        HookBody::Abstract => Value::Null,
        HookBody::Block(s) => json!(statements(s)),
        HookBody::Expression(e) => expression(e),
    };
    mnode(
        &h.meta,
        "PropertyHook",
        json!({"attrGroups":attributes(&h.attributes),"flags":integer(i64::from(h.flags.0)),"byRef":h.by_ref,"name":identifier(&h.name,false),"params":parameters(&h.params),"body":body}),
    )
}
fn class(c: &ClassDeclaration) -> Value {
    let mut fields = json!({"attrGroups":attributes(&c.attributes),"name":c.name.as_ref().map(|n|identifier(n,false)),"stmts":statements(&c.members)});
    let kind = match c.kind {
        ClassKind::Class => {
            fields["flags"] = integer(i64::from(c.flags.0));
            fields["extends"] = c.extends.first().map_or(Value::Null, name);
            fields["implements"] = json!(c.implements.iter().map(name).collect::<Vec<_>>());
            "Stmt_Class"
        }
        ClassKind::Interface => {
            fields["extends"] = json!(c.extends.iter().map(name).collect::<Vec<_>>());
            "Stmt_Interface"
        }
        ClassKind::Trait => "Stmt_Trait",
        ClassKind::Enum => {
            fields["scalarType"] = c.scalar_type.as_ref().map_or(Value::Null, ty);
            fields["implements"] = json!(c.implements.iter().map(name).collect::<Vec<_>>());
            "Stmt_Enum"
        }
    };
    node(kind, fields)
}
fn use_kind(k: UseKind) -> Value {
    integer(match k {
        UseKind::Inherited => 0,
        UseKind::Normal => 1,
        UseKind::Function => 2,
        UseKind::Constant => 3,
    })
}
fn adaptation(a: &TraitAdaptation) -> Value {
    match a {
        TraitAdaptation::Alias {
            trait_name,
            method,
            modifier,
            alias,
            meta,
        } => mnode(
            meta,
            "Stmt_TraitUseAdaptation_Alias",
            json!({"trait":trait_name.as_ref().map(name),"method":identifier(method,false),"newModifier":modifier.map(|m|integer(i64::from(m.0))),"newName":alias.as_ref().map(|n|identifier(n,false))}),
        ),
        TraitAdaptation::Precedence {
            trait_name,
            method,
            instead_of,
            meta,
        } => mnode(
            meta,
            "Stmt_TraitUseAdaptation_Precedence",
            json!({"trait":name(trait_name),"method":identifier(method,false),"insteadof":instead_of.iter().map(name).collect::<Vec<_>>()}),
        ),
    }
}
fn constants(c: &Constants, class: bool) -> Value {
    let mut fields = json!({"attrGroups":attributes(&c.attributes),"consts":c.constants.iter().map(|c|mnode(&c.meta,"Const",json!({"name":identifier(&c.name,false),"value":expression(&c.value)}))).collect::<Vec<_>>()});
    if class {
        fields["flags"] = integer(i64::from(c.flags.0));
        fields["type"] = c.ty.as_ref().map_or(Value::Null, ty);
    }
    node(
        if class {
            "Stmt_ClassConst"
        } else {
            "Stmt_Const"
        },
        fields,
    )
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
fn expression(value: &Expr) -> Value {
    with_meta(
        match &value.kind {
            ExprKind::NewAnonymous(c) => node(
                "Expr_New",
                json!({"class":with_meta(class(&c.declaration), &c.meta),"args":c.args.iter().map(arg).collect::<Vec<_>>()}),
            ),
            ExprKind::Closure(c) => node(
                "Expr_Closure",
                json!({"attrGroups":attributes(&c.attributes),"static":c.is_static,"byRef":c.signature.by_ref,"params":params(&c.signature),"uses":c.uses.iter().map(|u|mnode(&u.meta,"ClosureUse",json!({"var":expression(&u.variable),"byRef":u.by_ref}))).collect::<Vec<_>>(),"returnType":c.signature.return_type.as_ref().map(ty),"stmts":statements(&c.body)}),
            ),
            ExprKind::ArrowFunction(c) => node(
                "Expr_ArrowFunction",
                json!({"attrGroups":attributes(&c.attributes),"static":c.is_static,"byRef":c.signature.by_ref,"params":params(&c.signature),"returnType":c.signature.return_type.as_ref().map(ty),"expr":expression(&c.expr)}),
            ),
            ExprKind::Integer { value, .. } => node("Scalar_Int", json!({"value":integer(*value)})),
            ExprKind::Float(value) => node(
                "Scalar_Float",
                json!({"value":{"float64":format!("{:016x}",value.to_bits())}}),
            ),
            ExprKind::String { value, .. } => node("Scalar_String", json!({"value":bytes(value)})),
            ExprKind::Variable(value) => {
                node("Expr_Variable", json!({"name":bytes(value.bytes())}))
            }
            ExprKind::DynamicVariable(value) => {
                node("Expr_Variable", json!({"name":expression(value)}))
            }
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
                json!({"var":expression(target),"expr":expression(value)}),
            ),
            ExprKind::AssignRef { target, value } => node(
                "Expr_AssignRef",
                json!({"var":expression(target),"expr":expression(value)}),
            ),
            ExprKind::AssignOp { op, target, value } => node(
                &format!("Expr_AssignOp_{}", binary(*op)),
                json!({"var":expression(target),"expr":expression(value)}),
            ),
            ExprKind::Binary { op, left, right } => node(
                &format!("Expr_BinaryOp_{}", binary(*op)),
                json!({"left":expression(left),"right":expression(right)}),
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
                    json!({"var":expression(value)})
                } else {
                    json!({"expr":expression(value)})
                },
            ),
            ExprKind::Ternary {
                condition,
                then,
                otherwise,
            } => node(
                "Expr_Ternary",
                json!({"cond":expression(condition),"if":then.as_ref().map(|e|expression(e)),"else":expression(otherwise)}),
            ),
            ExprKind::Array { items: values, .. } => {
                node("Expr_Array", json!({"items":items(values)}))
            }
            ExprKind::List { items: values, .. } => {
                node("Expr_List", json!({"items":items(values)}))
            }
            ExprKind::ArrayDim { array, dim } => node(
                "Expr_ArrayDimFetch",
                json!({"var":expression(array),"dim":dim.as_ref().map(|e|expression(e))}),
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
                json!({"var":expression(object),"name":member(name,false)}),
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
                json!({"var":expression(object),"name":member(name,false),"args":args.iter().map(arg).collect::<Vec<_>>()}),
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
                json!({"expr":expression(value),"class":target(class)}),
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
                json!({"expr":expression(value)}),
            ),
            ExprKind::Include { kind, expr: value } => node(
                "Expr_Include",
                json!({"expr":expression(value),"type":integer(match kind {IncludeKind::Include=>1,IncludeKind::IncludeOnce=>2,IncludeKind::Require=>3,IncludeKind::RequireOnce=>4})}),
            ),
            ExprKind::Isset(values) => node(
                "Expr_Isset",
                json!({"vars":values.iter().map(expression).collect::<Vec<_>>()}),
            ),
            ExprKind::Exit(value) => node(
                "Expr_Exit",
                json!({"expr":value.as_ref().map(|e|expression(e))}),
            ),
            ExprKind::Yield { key, value } => node(
                "Expr_Yield",
                json!({"key":key.as_ref().map(|e|expression(e)),"value":value.as_ref().map(|e|expression(e))}),
            ),
            ExprKind::Match { condition, arms } => node(
                "Expr_Match",
                json!({"cond":expression(condition),"arms":arms.iter().map(|arm|mnode(&arm.meta,"MatchArm",json!({"conds":arm.conditions.as_ref().map(|c|c.iter().map(expression).collect::<Vec<_>>()),"body":expression(&arm.body)}))).collect::<Vec<_>>()}),
            ),
            ExprKind::Interpolated { parts: values, .. } => {
                node("Scalar_InterpolatedString", json!({"parts":parts(values)}))
            }
            ExprKind::ShellExec(values) => node("Expr_ShellExec", json!({"parts":parts(values)})),
            ExprKind::Error => node("Expr_Error", json!({})),
            _ => panic!("unmapped expression"),
        },
        &value.meta,
    )
}
fn statements(stmts: &[Stmt]) -> Vec<Value> {
    let mut out = Vec::new();
    for stmt in stmts {
        out.push(with_meta(match &stmt.kind {
            StmtKind::Namespace(n)=>node("Stmt_Namespace",json!({"name":n.name.as_ref().map(name),"stmts":n.body.as_ref().map(|b|statements(b))})),
            StmtKind::Use(u)=>{
                let mut fields=json!({"type":use_kind(u.kind),"uses":u.items.iter().map(|u|mnode(&u.meta,"UseItem",json!({"type":use_kind(u.kind),"name":name(&u.name),"alias":u.alias.as_ref().map(|a|identifier(a,false))}))).collect::<Vec<_>>()});
                if let Some(prefix)=&u.prefix {fields["prefix"]=name(prefix);}
                node(if u.prefix.is_some(){"Stmt_GroupUse"}else{"Stmt_Use"},fields)
            },
            StmtKind::Constants(c)=>constants(c,false), StmtKind::ClassConstants(c)=>constants(c,true),
            StmtKind::Class(c)=>class(c),
            StmtKind::Method(m)=>node("Stmt_ClassMethod",json!({"attrGroups":attributes(&m.attributes),"flags":integer(i64::from(m.flags.0)),"byRef":m.signature.by_ref,"name":identifier(&m.name,false),"params":params(&m.signature),"returnType":m.signature.return_type.as_ref().map(ty),"stmts":m.body.as_ref().map(|b|statements(b))})),
            StmtKind::Property(p)=>node("Stmt_Property",json!({"attrGroups":attributes(&p.attributes),"flags":integer(i64::from(p.flags.0)),"type":p.ty.as_ref().map(ty),"props":p.items.iter().map(|p|mnode(&p.meta,"PropertyItem",json!({"name":identifier(&p.name,true),"default":p.default.as_ref().map(|e|expression(e))}))).collect::<Vec<_>>(),"hooks":p.hooks.iter().map(hook).collect::<Vec<_>>()})),
            StmtKind::TraitUse(t)=>node("Stmt_TraitUse",json!({"traits":t.traits.iter().map(name).collect::<Vec<_>>(),"adaptations":t.adaptations.iter().map(adaptation).collect::<Vec<_>>()})),
            StmtKind::EnumCase(c)=>node("Stmt_EnumCase",json!({"attrGroups":attributes(&c.attributes),"name":identifier(&c.name,false),"expr":c.value.as_ref().map(|e|expression(e))})),
            StmtKind::Expression(value) => node("Stmt_Expression", json!({"expr":expression(value)})),
            StmtKind::Nop => node("Stmt_Nop", json!({})),
            StmtKind::InlineHtml(value) => node("Stmt_InlineHTML", json!({"value":bytes(value)})),
            StmtKind::Echo(values) => node(
                "Stmt_Echo",
                json!({"exprs":values.iter().map(expression).collect::<Vec<_>>()}),
            ),
            StmtKind::Return(value) => {
                node("Stmt_Return", json!({"expr":value.as_ref().map(expression)}))
            }
            StmtKind::Block(values) => node("Stmt_Block", json!({"stmts":statements(values)})),
            StmtKind::HaltCompiler(value) => {
                node("Stmt_HaltCompiler", json!({"remaining":bytes(value)}))
            }
            StmtKind::Function(f) => node("Stmt_Function",json!({"attrGroups":attributes(&f.attributes),"byRef":f.signature.by_ref,"name":identifier(&f.name,false),"params":params(&f.signature),"returnType":f.signature.return_type.as_ref().map(ty),"stmts":statements(&f.body)})),
            StmtKind::If(s) => node("Stmt_If",json!({"cond":expression(&s.condition),"stmts":statements(&s.body),"elseifs":s.elseifs.iter().map(|b|mnode(&b.meta,"Stmt_ElseIf",json!({"cond":expression(&b.condition),"stmts":statements(&b.body)}))).collect::<Vec<_>>(),"else":s.otherwise.as_ref().map(|b|mnode(&b.meta,"Stmt_Else",json!({"stmts":statements(&b.body)})))})),
            StmtKind::While(s) => node("Stmt_While",json!({"cond":expression(&s.condition),"stmts":statements(&s.body)})),
            StmtKind::Do(s) => node("Stmt_Do",json!({"cond":expression(&s.condition),"stmts":statements(&s.body)})),
            StmtKind::For(s) => node("Stmt_For",json!({"init":s.init.iter().map(expression).collect::<Vec<_>>(),"cond":s.conditions.iter().map(expression).collect::<Vec<_>>(),"loop":s.steps.iter().map(expression).collect::<Vec<_>>(),"stmts":statements(&s.body)})),
            StmtKind::Foreach(s) => node("Stmt_Foreach",json!({"expr":expression(&s.iterable),"keyVar":s.key.as_ref().map(|v|expression(v)),"valueVar":expression(&s.value),"byRef":s.by_ref,"stmts":statements(&s.body)})),
            StmtKind::Switch(s) => node("Stmt_Switch",json!({"cond":expression(&s.condition),"cases":s.cases.iter().map(|c|mnode(&c.meta,"Stmt_Case",json!({"cond":c.condition.as_ref().map(|e|expression(e)),"stmts":statements(&c.body)}))).collect::<Vec<_>>()})),
            StmtKind::Break(e) => node("Stmt_Break",json!({"num":e.as_ref().map(expression)})),
            StmtKind::Continue(e) => node("Stmt_Continue",json!({"num":e.as_ref().map(expression)})),
            StmtKind::Goto(n) => node("Stmt_Goto",json!({"name":identifier(n,false)})),
            StmtKind::Label(n) => node("Stmt_Label",json!({"name":identifier(n,false)})),
            StmtKind::Global(v) => node("Stmt_Global",json!({"vars":v.iter().map(expression).collect::<Vec<_>>()})),
            StmtKind::Unset(v) => node("Stmt_Unset",json!({"vars":v.iter().map(expression).collect::<Vec<_>>()})),
            StmtKind::Static(v) => node("Stmt_Static",json!({"vars":v.iter().map(|v|mnode(&v.meta,"StaticVar",json!({"var":expression(&v.variable),"default":v.default.as_ref().map(|e|expression(e))}))).collect::<Vec<_>>()})),
            StmtKind::Declare(s) => node("Stmt_Declare",json!({"declares":s.items.iter().map(|d|mnode(&d.meta,"DeclareItem",json!({"key":identifier(&d.key,false),"value":expression(&d.value)}))).collect::<Vec<_>>(),"stmts":s.body.as_ref().map(|b|statements(b))})),
            StmtKind::Try(s) => node("Stmt_TryCatch",json!({"stmts":statements(&s.body),"catches":s.catches.iter().map(|c|mnode(&c.meta,"Stmt_Catch",json!({"types":c.types.iter().map(name).collect::<Vec<_>>(),"var":c.variable.as_ref().map(|e|expression(e)),"stmts":statements(&c.body)}))).collect::<Vec<_>>(),"finally":s.finally.as_ref().map(|f|mnode(&f.meta,"Stmt_Finally",json!({"stmts":statements(&f.body)})))})),
            _ => panic!("unmapped statement"),
        }, &stmt.meta));
    }
    out
}
#[path = "metadata.rs"]
mod metadata;
fn with_meta(mut value: Value, meta: &NodeMeta) -> Value {
    value["_meta"] = metadata::raw(meta);
    value
}
fn mnode(meta: &NodeMeta, kind: &str, fields: Value) -> Value {
    with_meta(node(kind, fields), meta)
}
fn strip(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("_meta");
            for v in map.values_mut() {
                strip(v);
            }
        }
        Value::Array(values) => {
            for v in values {
                strip(v);
            }
        }
        _ => {}
    }
}
#[allow(dead_code)]
pub fn expr(value: &Expr) -> Value {
    let mut value = expression(value);
    strip(&mut value);
    value
}
#[allow(dead_code)]
pub fn observation(result: &ParseResult, target: &str) -> Value {
    let mut value = full_observation(result, target);
    value.as_object_mut().unwrap().remove("metadata");
    value
}
pub fn full_observation(result: &ParseResult, target: &str) -> Value {
    // The independent JSON oracle projection has much larger debug-build frames
    // than production parsing. Keep maximum-depth observations off small test
    // worker stacks; no stack growth or serialization dependency enters pprs.
    if result
        .program()
        .is_some_and(|p| pprs::visit::walk(p).any(|n| n.depth > 64))
    {
        return std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(8 * 1024 * 1024)
                .spawn_scoped(scope, || full_observation_inner(result, target))
                .unwrap()
                .join()
                .unwrap()
        });
    }
    full_observation_inner(result, target)
}
fn full_observation_inner(result: &ParseResult, target: &str) -> Value {
    let mut ast = result
        .program()
        .map_or(Value::Null, |p| json!(statements(&p.statements)));
    let metadata = metadata::extract(&mut ast, result);
    json!({"target":target,"status":match result.status(){ParseStatus::Clean=>"clean",ParseStatus::Recovered=>"recovered",ParseStatus::Aborted=>"aborted"},"ast":ast,"metadata":metadata})
}
