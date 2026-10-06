//! Friendly math input syntax translated to LaTeX for the ReX renderer.
//!
//! Accepted examples:
//!   sqrt(x+1)
//!   (a+b)/(c+d)
//!   sum(i^2,i,1,n)
//!   int(x^2,x,0,1)
//!   lim(sin(x)/x,x,0)
//!   vec(AB)
//! Raw LaTeX remains accepted too.

fn split_top_level_args(input: &str) -> Vec<String> {
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut out = Vec::new();
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(input[start..idx].trim().to_string());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    out.push(input[start..].trim().to_string());
    out
}

fn find_matching_outer_parens(s: &str) -> bool {
    if !s.starts_with('(') || !s.ends_with(')') {
        return false;
    }
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i + ch.len_utf8() < s.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn find_top_level_slash(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut result = None;
    for (idx, ch) in s.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '/' if depth == 0 => result = Some(idx),
            _ => {}
        }
    }
    result
}

fn call_body<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{}(", name);
    if s.starts_with(&prefix) && s.ends_with(')') {
        Some(&s[prefix.len()..s.len() - 1])
    } else {
        None
    }
}

fn render_call(name: &str, body: &str) -> Option<String> {
    let args = split_top_level_args(body);
    match name {
        "sqrt" if args.len() == 1 => Some(format!(r"\sqrt{{{}}}", convert_expr(&args[0]))),
        "vec" | "vector" if args.len() == 1 => {
            Some(format!(r"\vec{{{}}}", convert_expr(&args[0])))
        }
        "abs" if args.len() == 1 => Some(format!(r"\left|{}\right|", convert_expr(&args[0]))),
        "frac" if args.len() == 2 => Some(format!(
            r"\frac{{{}}}{{{}}}",
            convert_expr(&args[0]),
            convert_expr(&args[1])
        )),
        // Compact form: sum(i=1,n,i^2)
        "sum" if args.len() == 3 && args[0].contains('=') => Some(format!(
            r"\sum_{{{}}}^{{{}}} {}",
            convert_expr(&args[0]),
            convert_expr(&args[1]),
            convert_expr(&args[2])
        )),
        // CAS form: sum(i^2,i,1,n)
        "sum" if args.len() == 4 => Some(format!(
            r"\sum_{{{}={}}}^{{{}}} {}",
            convert_expr(&args[1]),
            convert_expr(&args[2]),
            convert_expr(&args[3]),
            convert_expr(&args[0])
        )),
        "prod" if args.len() == 4 => Some(format!(
            r"\prod_{{{}={}}}^{{{}}} {}",
            convert_expr(&args[1]),
            convert_expr(&args[2]),
            convert_expr(&args[3]),
            convert_expr(&args[0])
        )),
        // CAS/Maple-like form: int(x^2,x,0,1)
        "int" | "integral" if args.len() == 4 => Some(format!(
            r"\int_{{{}}}^{{{}}} {}\,d{}",
            convert_expr(&args[2]),
            convert_expr(&args[3]),
            convert_expr(&args[0]),
            convert_expr(&args[1])
        )),
        // GeoGebra-like shorthand: int(x^2,0,1), assumes x.
        "int" | "integral" if args.len() == 3 => Some(format!(
            r"\int_{{{}}}^{{{}}} {}\,dx",
            convert_expr(&args[1]),
            convert_expr(&args[2]),
            convert_expr(&args[0])
        )),
        // lim(x->0, sin(x)/x)
        "lim" | "limit" if args.len() == 2 => {
            let spec = args[0].replace("->", r"\to ");
            Some(format!(r"\lim_{{{}}} {}", convert_expr(&spec), convert_expr(&args[1])))
        }
        // lim(sin(x)/x,x,0)
        "lim" | "limit" if args.len() == 3 => Some(format!(
            r"\lim_{{{}\to {}}} {}",
            convert_expr(&args[1]),
            convert_expr(&args[2]),
            convert_expr(&args[0])
        )),
        "diff" | "derivative" if args.len() == 2 => Some(format!(
            r"\frac{{d}}{{d{}}}\left({}\right)",
            convert_expr(&args[1]),
            convert_expr(&args[0])
        )),
        "sin" | "cos" | "tan" | "ln" | "log" | "exp" if args.len() == 1 => Some(format!(
            r"\{}\left({}\right)",
            name,
            convert_expr(&args[0])
        )),
        _ => None,
    }
}

fn convert_expr(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return String::new();
    }

    if find_matching_outer_parens(s) {
        return format!(r"\left({}\right)", convert_expr(&s[1..s.len() - 1]));
    }

    if let Some(idx) = find_top_level_slash(s) {
        let left = &s[..idx];
        let right = &s[idx + 1..];
        return format!(
            r"\frac{{{}}}{{{}}}",
            convert_expr(left),
            convert_expr(right)
        );
    }

    for name in [
        "sqrt", "vec", "vector", "abs", "frac", "sum", "prod", "int", "integral", "lim",
        "limit", "diff", "derivative", "sin", "cos", "tan", "ln", "log", "exp",
    ] {
        if let Some(body) = call_body(s, name) {
            if let Some(rendered) = render_call(name, body) {
                return rendered;
            }
        }
    }

    let mut out = s.to_string();
    for (from, to) in [
        ("sqrt", r"\sqrt"),
        ("pi", r"\pi"),
        ("theta", r"\theta"),
        ("alpha", r"\alpha"),
        ("beta", r"\beta"),
        ("gamma", r"\gamma"),
        ("delta", r"\delta"),
        ("lambda", r"\lambda"),
        ("mu", r"\mu"),
        ("sigma", r"\sigma"),
        ("omega", r"\omega"),
    ] {
        if out == from {
            out = to.to_string();
        }
    }

    // Common Unicode powers/subscripts typed or pasted from course material.
    for (from, to) in [
        ("⁰", "^{0}"), ("¹", "^{1}"), ("²", "^{2}"), ("³", "^{3}"),
        ("⁴", "^{4}"), ("⁵", "^{5}"), ("⁶", "^{6}"), ("⁷", "^{7}"),
        ("⁸", "^{8}"), ("⁹", "^{9}"),
        ("₀", "_{0}"), ("₁", "_{1}"), ("₂", "_{2}"), ("₃", "_{3}"),
        ("₄", "_{4}"), ("₅", "_{5}"), ("₆", "_{6}"), ("₇", "_{7}"),
        ("₈", "_{8}"), ("₉", "_{9}"),
    ] {
        out = out.replace(from, to);
    }

    out = out
        .replace('π', r"\pi")
        .replace('θ', r"\theta")
        .replace('∞', r"\infty")
        .replace('≤', r"\le")
        .replace('≥', r"\ge")
        .replace('≠', r"\ne")
        .replace('→', r"\to")
        .replace('×', r"\times")
        .replace('·', r"\cdot");

    // GeoGebra/Maple-style parenthesized exponents/subscripts: x^(n+1), a_(i+1)
    out = out.replace("^(", "^{").replace("_(", "_{");
    if out.contains("^{") || out.contains("_{") {
        let mut rebuilt = String::new();
        let chars: Vec<char> = out.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            if i >= 2
                && chars[i] == ')'
                && ((chars[i - 1] != '\\') || i == chars.len() - 1)
                && rebuilt.matches('{').count() > rebuilt.matches('}').count()
            {
                rebuilt.push('}');
            } else {
                rebuilt.push(chars[i]);
            }
            i += 1;
        }
        out = rebuilt;
    }

    out
}

pub fn friendly_math_to_latex(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('\\') || trimmed.contains(r"\frac") || trimmed.contains(r"\sum") {
        return trimmed.to_string();
    }
    convert_expr(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_examples() {
        assert_eq!(friendly_math_to_latex("sqrt(x)"), r"\sqrt{x}");
        assert_eq!(
            friendly_math_to_latex("(a+b)/(c+d)"),
            r"\frac{\left(a+b\right)}{\left(c+d\right)}"
        );
        assert_eq!(
            friendly_math_to_latex("int(x^2,x,0,1)"),
            r"\int_{0}^{1} x^2\,dx"
        );
        assert_eq!(friendly_math_to_latex("vec(AB)"), r"\vec{AB}");
        assert_eq!(
            friendly_math_to_latex("sum(i^2,i,1,n)"),
            r"\sum_{i=1}^{n} i^2"
        );
        assert_eq!(
            friendly_math_to_latex("lim(sin(x)/x,x,0)"),
            r"\lim_{x\to 0} \frac{\sin\left(x\right)}{x}"
        );
        assert_eq!(friendly_math_to_latex("x²"), r"x^{2}");
    }
}
