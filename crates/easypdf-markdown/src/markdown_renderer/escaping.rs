pub(super) fn normalize_text(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n")
}

pub(super) fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

pub(super) fn escape_target(target: &str) -> String {
    target.replace(' ', "%20").replace(')', "%29")
}
