use {
    anyhow::Error,
    std::fmt,
};

/// Process exit category. Codes match the CLI contract: 1 failure, 2 usage, 3
/// blocked, 4 needs a prompt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ExitClass {
    Failure = 1,
    Usage = 2,
    Blocked = 3,
    NeedsPrompt = 4,
}

#[derive(Debug)]
pub struct Classified {
    pub class: ExitClass,
    message: String,
}

impl Classified {
    pub fn error(class: ExitClass, message: impl Into<String>) -> Error {
        Error::from(Self {
            class,
            message: message.into(),
        })
    }
}

impl fmt::Display for Classified {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Classified {}

pub fn usage(message: impl Into<String>) -> Error {
    Classified::error(ExitClass::Usage, message)
}

pub fn blocked(message: impl Into<String>) -> Error {
    Classified::error(ExitClass::Blocked, message)
}

pub fn needs_prompt(message: impl Into<String>) -> Error {
    Classified::error(ExitClass::NeedsPrompt, message)
}

pub fn exit_code(err: &Error) -> i32 {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<Classified>())
        .map(|classified| classified.class as i32)
        .unwrap_or(ExitClass::Failure as i32)
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        anyhow::Context,
    };

    #[test]
    fn exit_code_survives_context() {
        let err = Err::<(), _>(blocked("locked")).context("while reverting").unwrap_err();
        assert_eq!(exit_code(&err), 3);
        assert_eq!(exit_code(&anyhow::anyhow!("db")), 1);
    }
}
