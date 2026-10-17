//! Small deterministic grammar for structure-aware robustness campaigns.
pub fn program(data: &[u8]) -> String {
    use std::fmt::Write;
    fn expr(data: &[u8], at: &mut usize, depth: usize) -> String {
        let byte = data.get(*at).copied().unwrap_or(0);
        *at += 1;
        if depth == 0 || *at >= data.len() {
            return ["1", "$value", "'text'", "null"][usize::from(byte) % 4].into();
        }
        match byte % 6 {
            0 => format!(
                "({} + {})",
                expr(data, at, depth - 1),
                expr(data, at, depth - 1)
            ),
            1 => format!(
                "[{}, {}]",
                expr(data, at, depth - 1),
                expr(data, at, depth - 1)
            ),
            2 => format!("foo({})", expr(data, at, depth - 1)),
            3 => format!(
                "($value ? {} : {})",
                expr(data, at, depth - 1),
                expr(data, at, depth - 1)
            ),
            4 => format!("!({})", expr(data, at, depth - 1)),
            _ => "$object->property".into(),
        }
    }
    let mut source = String::from("<?php ");
    let mut at = 0;
    while at < data.len().min(128) {
        let selector = data[at];
        at += 1;
        let e = expr(data, &mut at, 5);
        match selector % 6 {
            0 => write!(&mut source, "$value = {e};"),
            1 => write!(&mut source, "if ({e}) {{ echo 1; }} else {{ echo 2; }}"),
            2 => write!(&mut source, "function f{at}($x = 1) {{ return {e}; }}"),
            3 => {
                write!(
                    &mut source,
                    "class C{at} {{ public $value; public function f($x) {{ return {e}; }} }}"
                )
            }
            4 => write!(&mut source, "foreach ([1, 2] as $value) {{ echo {e}; }}"),
            _ => write!(
                &mut source,
                "try {{ echo {e}; }} catch (Exception $e) {{ echo 1; }}"
            ),
        }
        .unwrap();
    }
    source
}
