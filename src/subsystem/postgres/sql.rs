use anyhow::Result;

pub(super) fn quote_ident(ident: &str) -> Result<String> {
    if ident.is_empty() || ident.contains('\0') {
        anyhow::bail!("SQL identifier cannot be empty or contain NUL");
    }
    let mut quoted = String::with_capacity(ident.len() + 2);
    quoted.push('"');
    for ch in ident.chars() {
        if ch == '"' {
            quoted.push('"');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    Ok(quoted)
}

pub(super) fn timeout_millis(seconds: u64) -> Result<i64> {
    let millis = seconds
        .checked_mul(1000)
        .ok_or_else(|| anyhow::anyhow!("timeout is too large"))?;
    i64::try_from(millis).map_err(|_| anyhow::anyhow!("timeout is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_escapes_identifiers() {
        assert_eq!(quote_ident("public").unwrap(), "\"public\"");
        assert_eq!(quote_ident("a\"b").unwrap(), "\"a\"\"b\"");
        assert!(quote_ident("").is_err());
    }
}
