use crate::config::TestGapConfig;
use crate::types::{AiAnalysis, TestGap, TokenUsage};
use crate::TestGapError;

const API_URL: &str = "https://api.anthropic.com/v1/messages";

pub async fn analyze_gaps(
    gaps: &mut [TestGap],
    config: &TestGapConfig,
) -> std::result::Result<TokenUsage, TestGapError> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        TestGapError::AiApi("ANTHROPIC_API_KEY not set. Use --no-ai to skip AI analysis.".into())
    })?;

    let client = reqwest::Client::new();
    let batch_size = config.ai.batch_size;
    let max_tokens = config.ai.max_function_body_tokens;
    let model = &config.ai.model;

    let mut total_input = 0u64;
    let mut total_output = 0u64;

    for chunk in gaps.chunks_mut(batch_size) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|gap| analyze_single_gap(&client, &api_key, model, max_tokens, gap))
            .collect();

        let results = futures::future::join_all(futures).await;

        for (gap, result) in chunk.iter_mut().zip(results) {
            match result {
                Ok((analysis, usage)) => {
                    total_input += usage.input_tokens;
                    total_output += usage.output_tokens;
                    gap.ai_analysis = Some(analysis);
                }
                Err(e) => {
                    tracing::warn!("AI analysis failed for {}: {e}", gap.function.name);
                }
            }
        }
    }

    Ok(TokenUsage {
        input_tokens: total_input,
        output_tokens: total_output,
    })
}

async fn analyze_single_gap(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    max_body_tokens: usize,
    gap: &TestGap,
) -> std::result::Result<(AiAnalysis, TokenUsage), TestGapError> {
    let body = truncate_body(&gap.function.body, max_body_tokens);

    let prompt = format!(
        r#"Analyze this untested {} function and provide test recommendations.

Function: {}
File: {}:{}
Visibility: {}
Complexity: {}

```{}
{}
```

Respond in this exact JSON format:
{{
  "risk_assessment": "brief risk description if this function has bugs",
  "suggested_tests": ["test case 1 description", "test case 2 description"],
  "priority_score": 7,
  "reasoning": "brief explanation of why these tests matter"
}}"#,
        gap.function.language,
        gap.function.name,
        gap.function.file_path.display(),
        gap.function.line_start,
        if gap.function.is_public {
            "public"
        } else {
            "private"
        },
        gap.function.complexity,
        gap.function.language,
        body,
    );

    let request_body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "system": "You are a senior software testing expert. Analyze functions and suggest specific, actionable test cases. Always respond with valid JSON only, no markdown fences.",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let resp = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| TestGapError::AiApi(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(TestGapError::AiApi(format!(
            "API returned {status}: {body}"
        )));
    }

    let response: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| TestGapError::AiApi(e.to_string()))?;

    let input_tokens = response["usage"]["input_tokens"].as_u64().unwrap_or(0);
    let output_tokens = response["usage"]["output_tokens"].as_u64().unwrap_or(0);
    let usage = TokenUsage {
        input_tokens,
        output_tokens,
    };

    // Extract text content from the response
    let text = response["content"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|block| block["text"].as_str())
        .unwrap_or("{}");

    // Parse the AI response JSON
    let analysis = parse_ai_response(text)?;

    Ok((analysis, usage))
}

pub(crate) fn parse_ai_response(text: &str) -> std::result::Result<AiAnalysis, TestGapError> {
    // Strip markdown fences if present
    let clean = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text.trim());
    let clean = clean.strip_suffix("```").unwrap_or(clean).trim();

    let value: serde_json::Value =
        serde_json::from_str(clean).map_err(|e| TestGapError::AiApi(format!("JSON parse: {e}")))?;

    Ok(AiAnalysis {
        risk_assessment: value["risk_assessment"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string(),
        suggested_tests: value["suggested_tests"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        priority_score: value["priority_score"].as_u64().unwrap_or(5) as u8,
        reasoning: value["reasoning"].as_str().unwrap_or("").to_string(),
    })
}

fn truncate_body(body: &str, max_tokens: usize) -> String {
    // Rough estimate: 1 token ≈ 4 characters
    let max_chars = max_tokens * 4;
    if body.len() <= max_chars {
        body.to_string()
    } else {
        format!("{}... (truncated)", &body[..max_chars])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_JSON: &str = r#"{
        "risk_assessment": "High risk if parsing fails silently",
        "suggested_tests": [
            "Test with empty input",
            "Test with malformed data"
        ],
        "priority_score": 8,
        "reasoning": "Core parsing logic needs thorough coverage"
    }"#;

    // ── valid JSON ──────────────────────────────────────────────────

    #[test]
    fn parse_valid_json() {
        let result = parse_ai_response(VALID_JSON);
        assert!(result.is_ok(), "expected Ok, got: {result:?}");

        let analysis = result.unwrap();
        assert_eq!(
            analysis.risk_assessment,
            "High risk if parsing fails silently"
        );
        assert_eq!(analysis.suggested_tests.len(), 2);
        assert_eq!(analysis.suggested_tests[0], "Test with empty input");
        assert_eq!(analysis.suggested_tests[1], "Test with malformed data");
        assert_eq!(analysis.priority_score, 8);
        assert_eq!(
            analysis.reasoning,
            "Core parsing logic needs thorough coverage"
        );
    }

    // ── JSON wrapped in markdown fences ─────────────────────────────

    #[test]
    fn parse_json_in_markdown_fences() {
        let fenced = format!("```json\n{}\n```", VALID_JSON);
        let result = parse_ai_response(&fenced);
        assert!(result.is_ok(), "expected Ok, got: {result:?}");

        let analysis = result.unwrap();
        assert_eq!(
            analysis.risk_assessment,
            "High risk if parsing fails silently"
        );
        assert_eq!(analysis.priority_score, 8);
        assert_eq!(analysis.suggested_tests.len(), 2);
    }

    #[test]
    fn parse_json_in_plain_fences() {
        let fenced = format!("```\n{}\n```", VALID_JSON);
        let result = parse_ai_response(&fenced);
        assert!(result.is_ok(), "expected Ok, got: {result:?}");

        let analysis = result.unwrap();
        assert_eq!(
            analysis.reasoning,
            "Core parsing logic needs thorough coverage"
        );
    }

    // ── malformed JSON ──────────────────────────────────────────────

    #[test]
    fn parse_malformed_json_returns_err() {
        let result = parse_ai_response("this is not json {{{");
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_string_returns_err() {
        let result = parse_ai_response("");
        assert!(result.is_err());
    }

    // ── missing fields fall back to defaults ────────────────────────

    #[test]
    fn parse_minimal_json_uses_defaults() {
        let minimal = r#"{}"#;
        let result = parse_ai_response(minimal);
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.risk_assessment, "Unknown");
        assert!(analysis.suggested_tests.is_empty());
        assert_eq!(analysis.priority_score, 5);
        assert_eq!(analysis.reasoning, "");
    }
}
