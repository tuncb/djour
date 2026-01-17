//! Tag query parsing and evaluation
//!
//! This module implements a boolean query system for filtering tagged content.
//! Supports AND, OR, and NOT operators with proper precedence.
//!
//! # Examples
//!
//! ```
//! use djour::domain::tags::TagQuery;
//!
//! let query = TagQuery::parse("work AND urgent").unwrap();
//! assert!(query.matches(&vec!["work".to_string(), "urgent".to_string()]));
//! ```

use crate::error::{DjourError, Result};
use std::collections::HashSet;

/// Tag query abstract syntax tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagQuery {
    /// Single tag (e.g., "work")
    Single(String),

    /// AND operation - both queries must match
    And(Box<TagQuery>, Box<TagQuery>),

    /// OR operation - either query must match
    Or(Box<TagQuery>, Box<TagQuery>),

    /// NOT operation - exclude matches
    Not(Box<TagQuery>),
}

impl TagQuery {
    /// Parse a query string into a TagQuery AST
    ///
    /// Supports: "tag", "tag1 AND tag2", "tag1 OR tag2", "tag NOT tag2"
    /// Operator precedence: NOT > AND > OR
    ///
    /// # Examples
    ///
    /// ```
    /// use djour::domain::tags::TagQuery;
    ///
    /// let query = TagQuery::parse("work").unwrap();
    /// assert!(query.matches(&vec!["work".to_string()]));
    ///
    /// let query = TagQuery::parse("work AND urgent").unwrap();
    /// assert!(query.matches(&vec!["work".to_string(), "urgent".to_string()]));
    /// ```
    pub fn parse(query: &str) -> Result<Self> {
        let tokens = tokenize(query)?;
        let mut pos = 0;
        let result = parse_or(&tokens, &mut pos)?;

        // Ensure all tokens were consumed
        if pos != tokens.len() {
            return Err(DjourError::Config(format!(
                "Unexpected tokens after position {}",
                pos
            )));
        }

        Ok(result)
    }

    /// Evaluate this query against a set of tags
    ///
    /// # Examples
    ///
    /// ```
    /// use djour::domain::tags::TagQuery;
    ///
    /// let query = TagQuery::parse("work AND urgent").unwrap();
    /// assert!(query.matches(&vec!["work".to_string(), "urgent".to_string()]));
    /// assert!(!query.matches(&vec!["work".to_string()]));
    /// ```
    pub fn matches(&self, tags: &[String]) -> bool {
        let tag_set: HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();
        self.matches_set(&tag_set)
    }

    /// Internal evaluation using HashSet for efficiency
    fn matches_set(&self, tags: &HashSet<&str>) -> bool {
        match self {
            TagQuery::Single(tag) => tags.contains(tag.as_str()),
            TagQuery::And(left, right) => left.matches_set(tags) && right.matches_set(tags),
            TagQuery::Or(left, right) => left.matches_set(tags) || right.matches_set(tags),
            TagQuery::Not(inner) => !inner.matches_set(tags),
        }
    }
}

impl std::fmt::Display for TagQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagQuery::Single(tag) => write!(f, "#{}", tag),
            TagQuery::And(left, right) => write!(f, "{} AND {}", left, right),
            TagQuery::Or(left, right) => write!(f, "({} OR {})", left, right),
            TagQuery::Not(inner) => write!(f, "NOT {}", inner),
        }
    }
}

/// Token types for query parsing
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Tag(String),
    And,
    Or,
    Not,
}

/// Tokenize a query string
fn tokenize(query: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let words: Vec<&str> = query.split_whitespace().collect();

    for word in words {
        match word.to_uppercase().as_str() {
            "AND" => tokens.push(Token::And),
            "OR" => tokens.push(Token::Or),
            "NOT" => tokens.push(Token::Not),
            _ => {
                // Remove leading # if present
                let tag = word.strip_prefix('#').unwrap_or(word);
                if tag.is_empty() {
                    return Err(DjourError::Config("Invalid tag in query".to_string()));
                }
                // Validate tag characters (alphanumeric, hyphens, underscores)
                if !tag
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    return Err(DjourError::Config(format!("Invalid tag: {}", tag)));
                }
                tokens.push(Token::Tag(tag.to_lowercase()));
            }
        }
    }

    if tokens.is_empty() {
        return Err(DjourError::Config("Empty query".to_string()));
    }

    Ok(tokens)
}

/// Parse OR expressions (lowest precedence)
fn parse_or(tokens: &[Token], pos: &mut usize) -> Result<TagQuery> {
    let mut left = parse_and(tokens, pos)?;

    while *pos < tokens.len() {
        if matches!(tokens[*pos], Token::Or) {
            *pos += 1; // consume OR
            let right = parse_and(tokens, pos)?;
            left = TagQuery::Or(Box::new(left), Box::new(right));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse AND expressions (medium precedence)
fn parse_and(tokens: &[Token], pos: &mut usize) -> Result<TagQuery> {
    let mut left = parse_not(tokens, pos)?;

    while *pos < tokens.len() {
        if matches!(tokens[*pos], Token::And) {
            *pos += 1; // consume AND
            let right = parse_not(tokens, pos)?;
            left = TagQuery::And(Box::new(left), Box::new(right));
        } else {
            break;
        }
    }

    Ok(left)
}

/// Parse NOT expressions (highest precedence)
fn parse_not(tokens: &[Token], pos: &mut usize) -> Result<TagQuery> {
    if *pos >= tokens.len() {
        return Err(DjourError::Config("Unexpected end of query".to_string()));
    }

    if matches!(tokens[*pos], Token::Not) {
        *pos += 1; // consume NOT
        let inner = parse_not(tokens, pos)?; // NOT is right-associative
        Ok(TagQuery::Not(Box::new(inner)))
    } else {
        parse_primary(tokens, pos)
    }
}

/// Parse primary expressions (tags)
fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<TagQuery> {
    if *pos >= tokens.len() {
        return Err(DjourError::Config("Unexpected end of query".to_string()));
    }

    match &tokens[*pos] {
        Token::Tag(tag) => {
            *pos += 1;
            Ok(TagQuery::Single(tag.clone()))
        }
        _ => Err(DjourError::Config(format!(
            "Expected tag, found {:?}",
            tokens[*pos]
        ))),
    }
}

#[cfg(test)]
#[allow(clippy::useless_vec)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_tag() {
        let query = TagQuery::parse("work").unwrap();
        assert_eq!(query, TagQuery::Single("work".to_string()));
    }

    #[test]
    fn test_parse_single_tag_with_hash() {
        let query = TagQuery::parse("#work").unwrap();
        assert_eq!(query, TagQuery::Single("work".to_string()));
    }

    #[test]
    fn test_parse_and() {
        let query = TagQuery::parse("work AND urgent").unwrap();
        assert_eq!(
            query,
            TagQuery::And(
                Box::new(TagQuery::Single("work".to_string())),
                Box::new(TagQuery::Single("urgent".to_string()))
            )
        );
    }

    #[test]
    fn test_parse_or() {
        let query = TagQuery::parse("work OR personal").unwrap();
        assert_eq!(
            query,
            TagQuery::Or(
                Box::new(TagQuery::Single("work".to_string())),
                Box::new(TagQuery::Single("personal".to_string()))
            )
        );
    }

    #[test]
    fn test_parse_not() {
        let query = TagQuery::parse("NOT meeting").unwrap();
        assert_eq!(
            query,
            TagQuery::Not(Box::new(TagQuery::Single("meeting".to_string())))
        );
    }

    #[test]
    fn test_parse_work_not_meeting() {
        let query = TagQuery::parse("work AND NOT meeting").unwrap();
        assert_eq!(
            query,
            TagQuery::And(
                Box::new(TagQuery::Single("work".to_string())),
                Box::new(TagQuery::Not(Box::new(TagQuery::Single(
                    "meeting".to_string()
                ))))
            )
        );
    }

    #[test]
    fn test_parse_complex() {
        // work AND urgent OR personal
        // Should parse as: work AND urgent OR personal (due to precedence)
        let query = TagQuery::parse("work AND urgent OR personal").unwrap();
        assert_eq!(
            query,
            TagQuery::Or(
                Box::new(TagQuery::And(
                    Box::new(TagQuery::Single("work".to_string())),
                    Box::new(TagQuery::Single("urgent".to_string()))
                )),
                Box::new(TagQuery::Single("personal".to_string()))
            )
        );
    }

    #[test]
    fn test_parse_case_insensitive_operators() {
        let query1 = TagQuery::parse("work and urgent").unwrap();
        let query2 = TagQuery::parse("work AND urgent").unwrap();
        assert_eq!(query1, query2);
    }

    #[test]
    fn test_parse_case_insensitive_tags() {
        let query = TagQuery::parse("WORK").unwrap();
        assert_eq!(query, TagQuery::Single("work".to_string()));
    }

    #[test]
    fn test_parse_empty_query() {
        let result = TagQuery::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = TagQuery::parse("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_tag() {
        let result = TagQuery::parse("work@email");
        assert!(result.is_err());
    }

    #[test]
    fn test_matches_single_tag() {
        let query = TagQuery::parse("work").unwrap();
        assert!(query.matches(&vec!["work".to_string()]));
        assert!(!query.matches(&vec!["personal".to_string()]));
    }

    #[test]
    fn test_matches_and() {
        let query = TagQuery::parse("work AND urgent").unwrap();
        assert!(query.matches(&vec!["work".to_string(), "urgent".to_string()]));
        assert!(!query.matches(&vec!["work".to_string()]));
        assert!(!query.matches(&vec!["urgent".to_string()]));
    }

    #[test]
    fn test_matches_or() {
        let query = TagQuery::parse("work OR personal").unwrap();
        assert!(query.matches(&vec!["work".to_string()]));
        assert!(query.matches(&vec!["personal".to_string()]));
        assert!(query.matches(&vec!["work".to_string(), "personal".to_string()]));
        assert!(!query.matches(&vec!["other".to_string()]));
    }

    #[test]
    fn test_matches_not() {
        let query = TagQuery::parse("work AND NOT meeting").unwrap();
        assert!(query.matches(&vec!["work".to_string(), "urgent".to_string()]));
        assert!(!query.matches(&vec!["work".to_string(), "meeting".to_string()]));
        assert!(!query.matches(&vec!["personal".to_string()]));
    }

    #[test]
    fn test_matches_complex() {
        let query = TagQuery::parse("work AND urgent OR personal").unwrap();
        // Should match: (work AND urgent) OR personal
        assert!(query.matches(&vec!["work".to_string(), "urgent".to_string()]));
        assert!(query.matches(&vec!["personal".to_string()]));
        assert!(!query.matches(&vec!["work".to_string()]));
    }

    #[test]
    fn test_to_string_single() {
        let query = TagQuery::parse("work").unwrap();
        assert_eq!(query.to_string(), "#work");
    }

    #[test]
    fn test_to_string_and() {
        let query = TagQuery::parse("work AND urgent").unwrap();
        assert_eq!(query.to_string(), "#work AND #urgent");
    }

    #[test]
    fn test_to_string_or() {
        let query = TagQuery::parse("work OR personal").unwrap();
        assert_eq!(query.to_string(), "(#work OR #personal)");
    }

    #[test]
    fn test_to_string_not() {
        let query = TagQuery::parse("work AND NOT meeting").unwrap();
        assert_eq!(query.to_string(), "#work AND NOT #meeting");
    }

    #[test]
    fn test_tags_with_hyphens_underscores() {
        let query = TagQuery::parse("project-alpha AND task_123").unwrap();
        assert!(query.matches(&vec!["project-alpha".to_string(), "task_123".to_string()]));
    }

    #[test]
    fn test_parse_multiple_ands() {
        let query = TagQuery::parse("work AND urgent AND important").unwrap();
        assert!(query.matches(&vec![
            "work".to_string(),
            "urgent".to_string(),
            "important".to_string()
        ]));
        assert!(!query.matches(&vec!["work".to_string(), "urgent".to_string()]));
    }

    #[test]
    fn test_parse_multiple_ors() {
        let query = TagQuery::parse("work OR personal OR hobby").unwrap();
        assert!(query.matches(&vec!["work".to_string()]));
        assert!(query.matches(&vec!["personal".to_string()]));
        assert!(query.matches(&vec!["hobby".to_string()]));
        assert!(!query.matches(&vec!["other".to_string()]));
    }

    #[test]
    fn test_parse_double_not() {
        let query = TagQuery::parse("NOT NOT work").unwrap();
        // Double negation should match work
        assert!(query.matches(&vec!["work".to_string()]));
        assert!(!query.matches(&vec!["other".to_string()]));
    }
}
