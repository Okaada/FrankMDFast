//! Rails (`i18n` gem) `%{name}` interpolation.

use crate::Args;
use crate::tree::Leaf;

/// Replace every `%{name}` whose `name` is in `args`; unknown tokens stay
/// verbatim (Rails would raise `MissingInterpolationArgument`). With at least
/// one argument, `%%` becomes `%`, as in `I18n.interpolate`; with none the
/// string is returned untouched (Rails skips interpolation entirely).
pub(crate) fn interpolate(message: &str, args: Args<'_>) -> String {
    if args.is_empty() {
        return message.to_owned();
    }
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(idx) = rest.find('%') {
        out.push_str(&rest[..idx]);
        let tail = &rest[idx..];
        if let Some(after) = tail.strip_prefix("%%") {
            out.push('%');
            rest = after;
        } else if let Some((name, after)) = token(tail) {
            match args.iter().find(|(k, _)| *k == name) {
                Some((_, value)) => out.push_str(value),
                None => out.push_str(&tail[..tail.len() - after.len()]),
            }
            rest = after;
        } else {
            out.push('%');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Parse `%{name}` (`\w+`, ASCII) at the start of `s`: the name and the rest.
fn token(s: &str) -> Option<(&str, &str)> {
    let inner = s.strip_prefix("%{")?;
    let end = inner.find('}')?;
    let name = &inner[..end];
    let valid = !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    valid.then(|| (name, &inner[end + 1..]))
}

/// `%{name}` tokens of a leaf, sorted and de-duplicated (the golden
/// generator's `interp_vars`: strings and arrays of strings).
pub(crate) fn leaf_vars(leaf: &Leaf) -> Vec<String> {
    let mut vars = Vec::new();
    collect(leaf, &mut vars);
    vars.sort();
    vars.dedup();
    vars
}

fn collect(leaf: &Leaf, out: &mut Vec<String>) {
    match leaf {
        Leaf::Str(s) => {
            let mut rest = s.as_str();
            while let Some(idx) = rest.find("%{") {
                match token(&rest[idx..]) {
                    Some((name, after)) => {
                        out.push(name.to_owned());
                        rest = after;
                    }
                    None => rest = &rest[idx + 2..],
                }
            }
        }
        Leaf::List(items) => items.iter().for_each(|item| collect(item, out)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_known_and_keeps_unknown_tokens() {
        assert_eq!(interpolate("%{a}-%{a}-%{b}", &[("a", "x")]), "x-x-%{b}");
        assert_eq!(interpolate("100%% %{a}", &[("a", "x")]), "100% x");
        assert_eq!(interpolate("100%% %{a}", &[]), "100%% %{a}");
    }

    #[test]
    fn collects_sorted_unique_vars() {
        assert_eq!(
            leaf_vars(&Leaf::Str("%{b} %{a} %{b} %{not valid}".into())),
            vec!["a", "b"]
        );
    }
}
