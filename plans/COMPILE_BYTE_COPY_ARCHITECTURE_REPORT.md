# Compile Pipeline Architecture Report: Span/Byte-Copy Extraction

Date: 2026-02-14
Owner: TBD
Status: Proposed (not implemented)

## 1. Goal

Evaluate and define an architecture change for `compile` so matched content can be emitted as raw source slices (byte-copy / span-copy) instead of reconstructed markdown fragments.

Primary intent:
- Preserve author formatting exactly when possible.
- Avoid formatting drift (for example, tight list items becoming loose).

## 2. Why This Is Not A Small Refactor

Current compile output is produced from parsed semantic units, not raw spans. This gives behavior beyond plain text extraction:

- Tag inheritance from headings/lists (`src/domain/tags/parser.rs`).
- Tag-only carrier paragraph support (`#work` paragraph tags next list without being emitted).
- Context-aware inclusion/exclusion rules for explicit tagged sections.
- Link/image path rewriting relative to `.compilations` output.
- Attachment of related blocks (for example paragraph + following fenced code block).

A pure byte copy by paragraph boundaries would lose or alter some of the above unless these behaviors are explicitly re-modeled.

## 3. Current Architecture (Summary)

- `TagParser::extract_from_markdown_for_output` produces `Vec<TaggedContent>` with normalized content text plus tags/context.
- `TagCompiler::filter` applies boolean query and de-duplication.
- `TagCompiler::to_markdown` renders final output with date/file grouping and separators.

Key constraint:
- `TaggedContent.content` is currently text-only payload; it does not keep source byte spans.

## 4. Target Architecture

Introduce a span-first extraction model with optional semantic fallback.

### 4.1 New Data Model

Add a source-backed representation:

```rust
pub struct SourceSpan {
    pub start: usize, // byte offset in source markdown
    pub end: usize,   // exclusive
}

pub enum ContentPayload {
    Span(SourceSpan),       // preferred path for raw copy
    Rendered(String),       // compatibility/fallback
}
```

Extend `TaggedContent` to carry:
- `payload: ContentPayload`
- enough metadata to preserve ordering and grouping (source file, date, context, tags).

### 4.2 Parser Changes

In parser:
- Track offsets from pulldown events into original markdown.
- Emit `Span` for paragraph/list/section candidates where exact source slicing is valid.
- Keep existing tag inheritance and match semantics unchanged.
- For cases where exact source slice cannot be represented safely, emit `Rendered(String)` fallback.

### 4.3 Compiler Changes

In compiler:
- For `Span`, slice source bytes and append raw text.
- For `Rendered`, keep current path.
- Continue query filtering and date/file grouping.
- Preserve separator logic, but prefer no injected formatting changes between adjacent slices from same source when boundaries are already explicit in source text.

### 4.4 Output Rewriting Layer

Path rewrite for markdown links/images currently occurs during extraction. With span-copy:
- Option A (recommended): rewrite only on copied slice text just before writing output.
- Option B: keep parser-time rewrite only for `Rendered` payload and introduce writer-time rewrite for `Span`.

Option A centralizes behavior and reduces parser complexity.

## 5. Behavioral Compatibility Matrix

Must remain unchanged:
- Tag matching semantics (single, AND/OR/NOT, inheritance).
- Section-tag behavior and de-duplication.
- Date/file grouping modes.
- `.compilations/<query>.md` naming rules.

May change by design:
- Formatting becomes closer to source for matched spans.
- Some current normalization side effects disappear (this is expected).

## 6. Migration Plan

Phase 1: Internal model prep
- Add `ContentPayload`, keep existing `content: String` temporarily.
- Add conversion helpers and compatibility shims.

Phase 2: Span capture
- Capture offsets for paragraph/list/section units.
- Emit span payloads where safe; fallback otherwise.

Phase 3: Writer path
- Teach compiler/writer to render span payloads from source text.
- Move link/image rewrite to writer layer for span payloads.

Phase 4: Cleanup
- Remove redundant reconstructed-path logic where span path fully covers behavior.
- Keep fallback route for edge cases.

Phase 5: Hardening
- Expand unit/integration/synthetic coverage.
- Validate no regressions in existing fixture suite.

## 7. Risks And Mitigations

Risk: Offset correctness with CRLF/LF and unicode boundaries.
- Mitigation: Use byte offsets only; avoid char indexing; add CRLF fixtures.

Risk: Pulldown event-to-source span ambiguity.
- Mitigation: Keep fallback `Rendered` payload; do not force 100% span coverage initially.

Risk: Link/image rewrite mismatch between rendered and span paths.
- Mitigation: Single writer-time rewrite function used by both payload types.

Risk: Regression in section/list inheritance semantics.
- Mitigation: Preserve filtering logic as-is; change payload transport only.

## 8. Test Strategy

Add tests at three levels:

1. Unit tests (`src/domain/tags/parser.rs`, `src/domain/tags/compiler.rs`)
- Span boundaries for paragraphs/lists/sections.
- Fallback path behavior.
- Mixed `Span` + `Rendered` payload rendering.

2. Integration tests (`tests/compile_tests.rs`)
- Byte-preserved tight list case.
- Paragraph + code fence adjacency.
- Link/image rewrite still correct.

3. Synthetic fixtures (`tests/fixtures/synthetic/*`)
- At least one dedicated case for byte-preserved list formatting.
- CRLF source file case if cross-platform-safe fixture handling allows.

## 9. Acceptance Criteria

Functional:
- Existing tests pass.
- New span-focused tests pass.
- No behavior regressions in query semantics and grouping.

Formatting:
- Tight list in source remains tight in compilation when matched as adjacent slices.
- No unexpected blank-line insertion between adjacent slices from same source context.

Operational:
- Performance is not materially worse on existing test corpus.

## 10. Estimated Scope

Rough size:
- Medium-to-large refactor (parser + compiler + tests).
- Estimated effort: 1-3 focused days depending on offset-capture complexity and fallback volume.

## 11. Recommended Implementation Order

1. Add `ContentPayload` and compatibility shims.
2. Implement span capture for easiest units first (plain paragraphs).
3. Add writer-time span rendering + path rewrite.
4. Extend to list items and sections.
5. Remove unnecessary string reconstruction once coverage is stable.

## 12. Open Questions

- Should adjacent spans ever be merged before output, or always emitted individually?
- Do we want exact byte preservation including trailing spaces, or normalized line endings?
- For mixed matched/unmatched blocks inside a section, should output preserve full local structure or only matched spans?
