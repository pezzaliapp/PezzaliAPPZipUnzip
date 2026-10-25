//! Errore unico restituito all'interfaccia, con un codice stabile
//! che il frontend traduce nella lingua dell'utente.

use serde::Serialize;

use crate::safety::Issue;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OpError {
    pub code: String,
    pub message: String,
    pub details: Vec<String>,
    /// Problemi di sicurezza rilevati nell'archivio (codice "unsafe").
    pub issues: Vec<Issue>,
}

impl OpError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self::with_details(code, message, Vec::new())
    }

    pub fn with_details(code: &str, message: impl Into<String>, details: Vec<String>) -> Self {
        OpError {
            code: code.to_string(),
            message: message.into(),
            details,
            issues: Vec::new(),
        }
    }

    pub fn engine(message: impl Into<String>) -> Self {
        Self::new("engine", message)
    }

    pub fn cancelled() -> Self {
        Self::new("cancelled", "Operazione annullata.")
    }

    pub fn io(context: &str, e: std::io::Error) -> Self {
        Self::with_details("io", context.to_string(), vec![e.to_string()])
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        for d in &self.details {
            write!(f, " — {d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for OpError {}
