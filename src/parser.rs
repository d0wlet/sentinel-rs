use crate::config::LogRule;
use crate::state::AppState;
use regex::RegexSet;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct PartialLog {
    level: Option<String>,
    severity: Option<String>,
    msg: Option<String>,
    message: Option<String>,
}

pub struct LogParser {
    // Optimization: RegexSet for checking ALL patterns in one pass
    regex_set: RegexSet,
    // Names correspond to indices in regex_set
    rule_names: Vec<String>,
}

impl LogParser {
    pub fn new(config_rules: &[LogRule]) -> Self {
        // Extract patterns strings
        let patterns: Vec<String> = config_rules.iter().map(|r| r.pattern.clone()).collect();

        let rule_names = config_rules.iter().map(|r| r.name.clone()).collect();

        let regex_set = RegexSet::new(&patterns).expect("Invalid RegexSet in config");

        Self {
            regex_set,
            rule_names,
        }
    }

    pub fn process_line(&self, line: &str, state: &Arc<AppState>) {
        state.increment_lines();

        // 1. Intelligent JSON Parsing (Star Material - Zero Lag Optimized)
        if line.trim_start().starts_with('{') {
            // Using PartialLog struct is faster than parsing into a Value map
            if let Ok(json) = serde_json::from_str::<PartialLog>(line) {
                let level_str = json.level.as_deref().or(json.severity.as_deref());

                let is_error = level_str
                    .map(|s: &str| {
                        let s = s.to_lowercase();
                        s == "error" || s == "panic" || s == "fatal"
                    })
                    .unwrap_or(false);

                if is_error {
                    // Try to format a nice message: "JSON Error: <msg>"
                    let msg = json
                        .message
                        .as_deref()
                        .or(json.msg.as_deref())
                        .unwrap_or(line);

                    state.record_error(format!("JSON: {}", msg));

                    // Webhook Trigger (Rate Limited)
                    if state.webhook_url.is_some() && state.should_send_webhook() {
                        let url = state.webhook_url.clone().unwrap();
                        let msg = msg.to_string();
                        tokio::spawn(async move {
                            let client = reqwest::Client::new();
                            let payload = serde_json::json!({
                                "text": format!("🚨 Sentinel Alert: JSON Error Detected!\nMessage: {}", msg)
                            });
                            let _ = client.post(&url).json(&payload).send().await;
                        });
                    }

                    return; // Early exit if JSON caught it
                }
            }
        }

        // 2. Fallback to RegexSet (Optimized)
        if self.regex_set.is_match(line) {
            // It matched SOMETHING. Now we check which one(s), or just take the first.
            let matches: Vec<_> = self.regex_set.matches(line).into_iter().collect();

            if !matches.is_empty() {
                // Just grab the name of the first match
                // In a real app we might handle multiple matches
                let idx = matches[0];
                let rule_name = &self.rule_names[idx];

                // If it looks like a severe error, we record it
                if rule_name.to_lowercase().contains("error")
                    || rule_name.to_lowercase().contains("panic")
                {
                    state.record_error(line.to_string());

                    // Webhook Trigger (Rate Limited)
                    if state.webhook_url.is_some() && state.should_send_webhook() {
                        let url = state.webhook_url.clone().unwrap();
                        let line = line.to_string();
                        tokio::spawn(async move {
                            let client = reqwest::Client::new();
                            let payload = serde_json::json!({
                                "text": format!("🚨 Sentinel Alert: Pattern Match!\nLog: {}", line)
                            });
                            let _ = client.post(&url).json(&payload).send().await;
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_process_line_regex_match() {
        let rules = vec![LogRule {
            name: "TestError".to_string(),
            pattern: "panic!".to_string(),
            threshold: 1,
        }];
        let parser = LogParser::new(&rules);
        let state = Arc::new(AppState::new(None));

        parser.process_line("System panic! at the disco", &state);

        assert_eq!(state.total_errors.load(Ordering::Relaxed), 1);
        assert_eq!(state.total_lines.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_process_line_json_error() {
        let rules = vec![];
        let parser = LogParser::new(&rules);
        let state = Arc::new(AppState::new(None));

        let json_log = r#"{"level": "error", "msg": "Database failed"}"#;
        parser.process_line(json_log, &state);

        assert_eq!(state.total_errors.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_process_line_benign() {
        let rules = vec![LogRule {
            name: "TestError".to_string(),
            pattern: "panic!".to_string(),
            threshold: 1,
        }];
        let parser = LogParser::new(&rules);
        let state = Arc::new(AppState::new(None));

        parser.process_line("Just a normal info log", &state);
        
        assert_eq!(state.total_errors.load(Ordering::Relaxed), 0);
        assert_eq!(state.total_lines.load(Ordering::Relaxed), 1);
    }
}
