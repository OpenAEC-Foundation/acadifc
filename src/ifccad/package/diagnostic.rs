use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PackageDiagnosticContextValue {
    Null,
    Boolean(bool),
    Number(serde_json::Number),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageDiagnostic {
    pub code: String,
    pub severity: PackageDiagnosticSeverity,
    pub resource_uri: Option<String>,
    pub location: Option<String>,
    pub context: BTreeMap<String, PackageDiagnosticContextValue>,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageValidationReport {
    pub diagnostics: Vec<PackageDiagnostic>,
}

impl PackageValidationReport {
    pub(crate) fn from_diagnostics(mut diagnostics: Vec<PackageDiagnostic>) -> Self {
        diagnostics
            .sort_by(|left, right| diagnostic_sort_key(left).cmp(&diagnostic_sort_key(right)));
        Self { diagnostics }
    }

    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|item| item.severity == PackageDiagnosticSeverity::Error)
    }
}

fn diagnostic_sort_key(diagnostic: &PackageDiagnostic) -> (&str, &str, u8, &str, String) {
    (
        diagnostic.resource_uri.as_deref().unwrap_or(""),
        diagnostic.location.as_deref().unwrap_or(""),
        severity_rank(diagnostic.severity),
        &diagnostic.code,
        serde_json::to_string(&diagnostic.context)
            .expect("diagnostic scalar context always serializes"),
    )
}

fn severity_rank(severity: PackageDiagnosticSeverity) -> u8 {
    match severity {
        PackageDiagnosticSeverity::Error => 0,
        PackageDiagnosticSeverity::Warning => 1,
        PackageDiagnosticSeverity::Info => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn diagnostic(
        code: &str,
        severity: PackageDiagnosticSeverity,
        resource_uri: Option<&str>,
        location: Option<&str>,
    ) -> PackageDiagnostic {
        PackageDiagnostic {
            code: code.to_owned(),
            severity,
            resource_uri: resource_uri.map(str::to_owned),
            location: location.map(str::to_owned),
            context: BTreeMap::new(),
            message: String::new(),
        }
    }

    fn diagnostic_with_context(
        code: &str,
        context: BTreeMap<String, PackageDiagnosticContextValue>,
    ) -> PackageDiagnostic {
        PackageDiagnostic {
            code: code.to_owned(),
            severity: PackageDiagnosticSeverity::Error,
            resource_uri: Some("resource.json".to_owned()),
            location: Some("/value".to_owned()),
            context,
            message: String::new(),
        }
    }

    #[test]
    fn report_is_valid_without_error_diagnostics() {
        let report = PackageValidationReport::from_diagnostics(vec![diagnostic(
            "TEST_WARNING",
            PackageDiagnosticSeverity::Warning,
            None,
            None,
        )]);

        assert!(report.is_valid());
    }

    #[test]
    fn report_is_invalid_with_an_error_diagnostic() {
        let report = PackageValidationReport::from_diagnostics(vec![diagnostic(
            "TEST_ERROR",
            PackageDiagnosticSeverity::Error,
            None,
            None,
        )]);

        assert!(!report.is_valid());
    }

    #[test]
    fn diagnostics_are_sorted_by_the_language_neutral_contract() {
        let report = PackageValidationReport::from_diagnostics(vec![
            diagnostic(
                "Z",
                PackageDiagnosticSeverity::Info,
                Some("z.json"),
                Some("/b"),
            ),
            diagnostic(
                "B",
                PackageDiagnosticSeverity::Warning,
                Some("a.json"),
                Some("/a"),
            ),
            diagnostic(
                "A",
                PackageDiagnosticSeverity::Error,
                Some("a.json"),
                Some("/a"),
            ),
        ]);

        let codes: Vec<_> = report
            .diagnostics
            .iter()
            .map(|item| item.code.as_str())
            .collect();
        assert_eq!(codes, ["A", "B", "Z"]);
    }

    #[test]
    fn context_breaks_otherwise_equal_sort_keys_canonically() {
        let mut later_context = BTreeMap::new();
        later_context.insert(
            "z".to_owned(),
            PackageDiagnosticContextValue::Number(0.into()),
        );
        later_context.insert(
            "a".to_owned(),
            PackageDiagnosticContextValue::String("zeta".to_owned()),
        );
        let mut earlier_context = BTreeMap::new();
        earlier_context.insert(
            "a".to_owned(),
            PackageDiagnosticContextValue::String("alpha".to_owned()),
        );
        earlier_context.insert(
            "z".to_owned(),
            PackageDiagnosticContextValue::Number(0.into()),
        );

        let report = PackageValidationReport::from_diagnostics(vec![
            diagnostic_with_context("SAME", later_context),
            diagnostic_with_context("SAME", earlier_context.clone()),
        ]);

        assert_eq!(report.diagnostics[0].context, earlier_context);
    }
}
