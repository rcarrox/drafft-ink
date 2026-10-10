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
        "root" if args.len() == 2 => Some(format!(
            r"\sqrt[{}]{{{}}}",
            convert_expr(&args[1]),
            convert_expr(&args[0])
        )),
        "sqrt" if args.len() == 1 => Some(format!(r"\sqrt{{{}}}", convert_expr(&args[0]))),
        "vec" | "vector" if args.len() == 1 => Some(format!(r"\vec{{{}}}", convert_expr(&args[0]))),
        "abs" if args.len() == 1 => Some(format!(r"\left|{}\right|", convert_expr(&args[0]))),
        "bin" if args.len() == 2 => Some(format!(
            r"\binom{{{}}}{{{}}}",
            convert_expr(&args[0]),
            convert_expr(&args[1])
        )),
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
            Some(format!(
                r"\lim_{{{}}} {}",
                convert_expr(&spec),
                convert_expr(&args[1])
            ))
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
        "sin" | "cos" | "tan" | "ln" | "log" | "exp" if args.len() == 1 => {
            Some(format!(r"\{}\left({}\right)", name, convert_expr(&args[0])))
        }
        _ => None,
    }
}

fn convert_atom_text(input: &str) -> String {
    let mut out = input.to_string();

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

    out = out
        .replace('π', r"\pi ")
        .replace('θ', r"\theta ")
        .replace('∞', r"\infty ")
        .replace('≤', r"\le ")
        .replace('≥', r"\ge ")
        .replace('≠', r"\ne ")
        .replace('→', r"\to ")
        .replace('×', r"\times ")
        .replace('·', r"\cdot ");

    out
}

#[derive(Debug)]
struct StructuredRow {
    latex: String,
    next: usize,
}

/// Parse the friendly input as a tiny structured editor language.
/// Slash, caret and underscore open a child block. One SPACE closes exactly
/// one block, so nested fractions/exponents can be left one level at a time.
fn parse_structured_row(chars: &[char], mut i: usize, stop_on_space: bool) -> StructuredRow {
    let mut items: Vec<(String, bool)> = Vec::new();

    while i < chars.len() {
        let ch = chars[i];

        if ch.is_whitespace() {
            i += 1;
            if stop_on_space {
                break;
            }
            continue;
        }

        if ch == '(' {
            let start = i + 1;
            let mut depth = 1i32;
            i += 1;
            while i < chars.len() && depth > 0 {
                match chars[i] {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    i += 1;
                }
            }
            let inner: String = chars[start..i.min(chars.len())].iter().collect();
            if i < chars.len() && chars[i] == ')' {
                i += 1;
            }
            items.push((format!(r"\left({}\right)", convert_expr(&inner)), true));
            continue;
        }

        if ch.is_alphabetic() {
            let ident_start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[ident_start..i].iter().collect();
            if i < chars.len() && chars[i] == '(' {
                let body_start = i + 1;
                let mut depth = 1i32;
                i += 1;
                while i < chars.len() && depth > 0 {
                    match chars[i] {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                    if depth > 0 {
                        i += 1;
                    }
                }
                let body: String = chars[body_start..i.min(chars.len())].iter().collect();
                if i < chars.len() && chars[i] == ')' {
                    i += 1;
                }
                if let Some(rendered) = render_call(&ident, &body) {
                    items.push((rendered, true));
                } else {
                    items.push((
                        format!(
                            r"{}\left({}\right)",
                            convert_atom_text(&ident),
                            convert_expr(&body)
                        ),
                        true,
                    ));
                }
                continue;
            }

            items.push((convert_atom_text(&ident), true));
            continue;
        }

        if ch.is_ascii_digit() || ch == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let atom: String = chars[start..i].iter().collect();
            items.push((atom, true));
            continue;
        }

        match ch {
            '/' => {
                i += 1;
                let numerator = items
                    .iter()
                    .rposition(|(_, is_operand)| *is_operand)
                    .map(|idx| items.remove(idx).0)
                    .unwrap_or_else(|| r"\;".to_string());
                let denom = parse_structured_row(chars, i, true);
                i = denom.next;
                let denominator = if denom.latex.trim().is_empty() {
                    r"\;".to_string()
                } else {
                    denom.latex
                };
                items.push((format!(r"\frac{{{}}}{{{}}}", numerator, denominator), true));
            }
            '^' | '_' => {
                let op = ch;
                i += 1;
                let base = items
                    .iter()
                    .rposition(|(_, is_operand)| *is_operand)
                    .map(|idx| items.remove(idx).0)
                    .unwrap_or_else(|| r"\;".to_string());
                let child = parse_structured_row(chars, i, true);
                i = child.next;
                let body = if child.latex.trim().is_empty() {
                    r"\;".to_string()
                } else {
                    child.latex
                };
                if op == '^' {
                    items.push((format!(r"{}^{{{}}}", base, body), true));
                } else {
                    items.push((format!(r"{}_{{{}}}", base, body), true));
                }
            }
            '+' | '-' | '=' => {
                items.push((ch.to_string(), false));
                i += 1;
            }
            '*' => {
                items.push((r"\cdot ".to_string(), false));
                i += 1;
            }
            ',' => {
                items.push((", ".to_string(), false));
                i += 1;
            }
            ')' => break,
            _ => {
                items.push((convert_atom_text(&ch.to_string()), true));
                i += 1;
            }
        }
    }

    StructuredRow {
        latex: items.into_iter().map(|(s, _)| s).collect::<String>(),
        next: i,
    }
}

fn convert_expr(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    parse_structured_row(&chars, 0, false).latex
}

/// Normalize Unicode powers/subscripts (including text expanders such as
/// Beeptexte) into ASCII structured notation. The added SPACE closes the block.
pub fn normalize_friendly_math_input(input: &str) -> String {
    fn super_digit(ch: char) -> Option<char> {
        match ch {
            '⁰' => Some('0'),
            '¹' => Some('1'),
            '²' => Some('2'),
            '³' => Some('3'),
            '⁴' => Some('4'),
            '⁵' => Some('5'),
            '⁶' => Some('6'),
            '⁷' => Some('7'),
            '⁸' => Some('8'),
            '⁹' => Some('9'),
            'ⁿ' => Some('n'),
            _ => None,
        }
    }
    fn sub_digit(ch: char) -> Option<char> {
        match ch {
            '₀' => Some('0'),
            '₁' => Some('1'),
            '₂' => Some('2'),
            '₃' => Some('3'),
            '₄' => Some('4'),
            '₅' => Some('5'),
            '₆' => Some('6'),
            '₇' => Some('7'),
            '₈' => Some('8'),
            '₉' => Some('9'),
            'ₙ' => Some('n'),
            _ => None,
        }
    }

    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len() + 8);
    let mut i = 0usize;

    while i < chars.len() {
        if let Some(d) = super_digit(chars[i]) {
            out.push('^');
            out.push(d);
            i += 1;
            while i < chars.len() {
                if let Some(next) = super_digit(chars[i]) {
                    out.push(next);
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(' ');
            continue;
        }

        if let Some(d) = sub_digit(chars[i]) {
            out.push('_');
            out.push(d);
            i += 1;
            while i < chars.len() {
                if let Some(next) = sub_digit(chars[i]) {
                    out.push(next);
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(' ');
            continue;
        }

        match chars[i] {
            '⁺' => out.push_str("^+ "),
            '⁻' => out.push_str("^- "),
            '₊' => out.push_str("_+ "),
            '₋' => out.push_str("_- "),
            ch => out.push(ch),
        }
        i += 1;
    }

    out
}

/// Number of structured blocks still open at the end of the source.
pub fn open_structured_depth(input: &str) -> usize {
    let mut depth = 0usize;
    for ch in input.chars() {
        match ch {
            '/' | '^' | '_' => depth += 1,
            ' ' | '\t' if depth > 0 => depth -= 1,
            ',' | ')' => depth = 0,
            _ => {}
        }
    }
    depth
}

pub fn friendly_math_to_latex(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with('\\') || trimmed.contains(r"\frac") || trimmed.contains(r"\sum") {
        return trimmed
            .replace('∞', r"\infty ")
            .replace('≤', r"\le ")
            .replace('≥', r"\ge ");
    }
    let normalized = normalize_friendly_math_input(input);
    convert_expr(normalized.trim())
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
            r"\int_{0}^{1} x^{2}\,dx"
        );
        assert_eq!(friendly_math_to_latex("vec(AB)"), r"\vec{AB}");
        assert_eq!(
            friendly_math_to_latex("sum(i^2,i,1,n)"),
            r"\sum_{i=1}^{n} i^{2}"
        );
        assert_eq!(
            friendly_math_to_latex("lim(sin(x)/x,x,0)"),
            r"\lim_{x\to 0} \frac{\sin\left(x\right)}{x}"
        );
        assert_eq!(friendly_math_to_latex("x²"), r"x^{2}");
        assert_eq!(normalize_friendly_math_input("x⁴+1"), "x^4 +1");
        assert_eq!(friendly_math_to_latex("x⁴+1"), r"x^{4}+1");
        assert_eq!(friendly_math_to_latex("x^2 +3"), r"x^{2}+3");
        assert_eq!(friendly_math_to_latex("1/2 +3"), r"\frac{1}{2}+3");
        assert_eq!(
            friendly_math_to_latex("1/2/3 +4 +5"),
            r"\frac{1}{\frac{2}{3}+4}+5"
        );
        assert_eq!(open_structured_depth("1/2/3"), 2);
        assert_eq!(open_structured_depth("1/2/3 "), 1);
        assert_eq!(open_structured_depth("1/2/3  "), 0);
    }
}

/// The dead key has already opened an exponent block. Browser composition may
/// echo its caret or compose a circumflex vowel; retain only the intended text.
pub fn dead_caret_text(text: &str) -> String {
    let text = text.strip_prefix('^').unwrap_or(text);
    text.chars()
        .map(|c| match c {
            'â' => 'a',
            'ê' => 'e',
            'î' => 'i',
            'ô' => 'o',
            'û' => 'u',
            'Â' => 'A',
            'Ê' => 'E',
            'Î' => 'I',
            'Ô' => 'O',
            'Û' => 'U',
            c => c,
        })
        .collect()
}

/// Match a complete command prefix immediately before the text caret.
pub fn text_command_prefix(text: &str, caret: usize) -> Option<(usize, &'static str)> {
    let prefix = text.get(..caret)?;
    for name in ["sum", "prod", "int", "lim", "sqrt", "root", "frac", "bin"] {
        let marker = format!("{name}(");
        if prefix.ends_with(&marker) {
            let start = prefix.len() - marker.len();
            if prefix[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_')
            {
                return Some((start, name));
            }
        }
    }
    None
}

/// Complete only the preview, leaving the user's source/caret untouched.
/// Empty arguments use a visible dot; nested commands remain recursive.
pub fn live_command_latex(source: &str) -> Option<String> {
    let source = normalize_friendly_math_input(source);
    let source = source.trim();
    let open = source.find('(')?;
    let name = &source[..open];
    let arity = match name {
        "sum" | "prod" | "int" => 4,
        "lim" => 3,
        "sqrt" => 1,
        "frac" | "root" | "bin" => 2,
        _ => return None,
    };
    let mut completed = source.to_string();
    let mut depth = 0i32;
    for ch in source.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    completed.extend(std::iter::repeat_n(')', depth as usize));
    let body = completed.get(open + 1..completed.len().checked_sub(1)?)?;
    let mut args = split_top_level_args(body);
    if args.len() > arity {
        return None;
    }
    args.resize(arity, String::new());
    for argument in &mut args {
        if argument.trim().is_empty() {
            *argument = r"\cdot".into();
        } else if argument.trim() == "\u{e000}" {
            *argument = "\u{e000}\\cdot".into();
        } else if let Some(nested) = live_command_latex(argument) {
            *argument = nested;
        }
    }
    render_call(name, &args.join(","))
}

#[cfg(test)]
mod live_text_command_tests {
    use super::*;
    #[test]
    fn prefixes_respect_word_boundaries_and_unicode() {
        assert_eq!(text_command_prefix("é = sum(", 9), Some((5, "sum")));
        assert_eq!(text_command_prefix("consum(", 7), None);
        for command in ["sum(", "prod(", "int(", "lim(", "sqrt(", "frac("] {
            assert!(text_command_prefix(command, command.len()).is_some());
            assert!(live_command_latex(command).is_some());
        }
    }
    #[test]
    fn live_commands_preserve_nested_fractions_and_superscripts() {
        for source in [
            "sum(kx,k,1,n)",
            "prod(k,k,1,n)",
            "int(x²,x,1,2)",
            "lim(x,x,5)",
            "sqrt(x²)",
            "frac(a,b)",
            "frac(a,frac(b,c))",
        ] {
            assert_eq!(
                live_command_latex(source).unwrap(),
                friendly_math_to_latex(source)
            );
        }
        let preview = live_command_latex("frac(a,frac(b,").unwrap();
        assert!(preview.contains(r"\frac{a}{\frac{b}{\cdot}}"));
        assert!(live_command_latex("frac(a,b,c)").is_none());
    }
}

#[cfg(test)]
mod binomial_and_infinity_tests {
    use super::*;
    #[test]
    fn binomial_live_and_nested_arguments() {
        assert_eq!(text_command_prefix("= bin(", 6), Some((2, "bin")));
        assert_eq!(friendly_math_to_latex("bin(n,k)"), r"\binom{n}{k}");
        assert_eq!(
            live_command_latex("bin("),
            Some(r"\binom{\cdot}{\cdot}".into())
        );
        assert_eq!(
            friendly_math_to_latex("bin(frac(n,2),k)"),
            r"\binom{\frac{n}{2}}{k}"
        );
    }
    #[test]
    fn unicode_bounds_are_normalized_even_in_latex() {
        assert!(friendly_math_to_latex("int(x,x,0,∞)").contains(r"\infty"));
        assert_eq!(
            friendly_math_to_latex(r"\int_{0}^{∞} x"),
            r"\int_{0}^{\infty } x"
        );
    }
}

#[cfg(test)]
mod live_root_tests {
    #[test]
    fn root_prefix_nested_preview() {
        assert_eq!(
            super::text_command_prefix("root(", 5),
            Some((0, "root".into()))
        );
        assert_eq!(
            super::friendly_math_to_latex("root(frac(a,b),3)"),
            r"\sqrt[3]{\frac{a}{b}}"
        );
    }
}

pub fn command_example(source: &str) -> Option<&'static str> {
    match source.split_once('(')?.0.trim() {
        "frac" => Some("frac(x+1,8)"),
        "sqrt" => Some("sqrt(x+1)"),
        "root" => Some("root(x,3)"),
        "sum" => Some("sum(kx,k,1,n)"),
        "prod" => Some("prod(k,k,1,n)"),
        "int" => Some("int(x²,x,1,2)"),
        "lim" => Some("lim(x,x,5)"),
        "bin" => Some("bin(n,k)"),
        _ => None,
    }
}

/// A zero-width layout probe, never stored in the document or exported.
pub fn live_command_caret_latex(source: &str, character: usize) -> Option<String> {
    const MARKER: &str = r"\color{red}{\rule{0em}{0.0001em}}";
    let length = source.chars().count();
    if character >= length && source.trim_end().ends_with(')') {
        return Some(format!("{}{}", live_command_latex(source)?, MARKER));
    }
    let mut character = character.min(length);
    let chars: Vec<char> = source.chars().collect();
    let mut start = 0;
    while start < chars.len() {
        if chars[start].is_ascii_alphabetic() {
            let mut end = start + 1;
            while end < chars.len() && chars[end].is_ascii_alphabetic() { end += 1; }
            if chars.get(end) == Some(&'(') && character >= start && character <= end {
                character = end + 1;
            }
            start = end;
        } else { start += 1; }
    }
    let byte = source.char_indices().nth(character).map(|(byte,_)|byte).unwrap_or(source.len());
    let mut marked = source.to_string();
    marked.insert(byte, '\u{e000}');
    let latex = live_command_latex(&marked)?;
    Some(latex.replace('\u{e000}', MARKER))
}

pub fn command_caret_candidates(source: &str) -> Vec<usize> {
    let chars: Vec<char> = source.chars().collect();
    let mut hidden = std::collections::HashSet::new();
    let mut start = 0;
    while start < chars.len() {
        if chars[start].is_ascii_alphabetic() {
            let mut end = start + 1;
            while end < chars.len() && chars[end].is_ascii_alphabetic() { end += 1; }
            if chars.get(end) == Some(&'(') { hidden.extend(start..=end); }
            start = end;
        } else { start += 1; }
    }
    (0..=chars.len()).filter(|i| !hidden.contains(i)).collect()
}

/// Innermost argument containing the source caret, including unfinished commands.
pub fn command_active_range(source: &str, caret: usize) -> Option<std::ops::Range<usize>> {
    let mut stack = Vec::new();
    let mut candidates = Vec::new();
    let length = source.chars().count();
    for (index, c) in source.chars().enumerate() {
        match c {
            '(' => stack.push(index + 1),
            ',' => if let Some(start) = stack.last_mut() { candidates.push(*start..index); *start = index + 1; },
            ')' => if let Some(start) = stack.pop() { candidates.push(start..index); },
            _ => {},
        }
    }
    candidates.extend(stack.into_iter().map(|start|start..length));
    candidates.into_iter().filter(|range|caret>=range.start&&caret<=range.end).min_by_key(|range|range.len())
}

#[cfg(test)]
mod live_caret_tests {
    use super::*;
    #[test]
    fn probe_follows_the_source_without_altering_it() {
        let source = "frac(123,7895)";
        let caret = source.find('5').unwrap();
        let probe = live_command_caret_latex(source,caret).unwrap();
        assert!(probe.contains(r"789\color{red}{\rule{0em}{0.0001em}}5"));
        let nested = live_command_caret_latex("frac(1,frac(π,95))",14).unwrap();
        assert!(nested.contains("red"));
        assert_eq!(source,"frac(123,7895)");
        assert!(!command_caret_candidates(source).contains(&2));
        assert!(command_caret_candidates(source).contains(&caret));
    }
}

#[cfg(test)]
mod example_tests {
    #[test]
    fn examples_match_active_command() {
        assert_eq!(super::command_example("frac("), Some("frac(x+1,8)"));
        assert_eq!(super::command_example("root(x,"), Some("root(x,3)"));
        assert_eq!(super::command_example("unknown("), None);
    }
}
