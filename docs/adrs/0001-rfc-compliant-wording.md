# ADR-0001: RFC 2119 Compliant Wording

## Status

Accepted

## Date

2025-01-27T20:45:00Z

## Context

Technical documentation and specifications within Athena need to clearly communicate the level of requirement for various practices, configurations, and implementations. Without standardized language, there can be ambiguity about whether something is mandatory, recommended, or optional.

RFC 2119 defines key words that are used in technical documents to indicate requirement levels. These key words provide precise meaning and reduce ambiguity in specifications.

## Decision

All technical documentation, code comments, and specifications within the Athena project MUST use RFC 2119 compliant language when expressing requirements.

The key words defined in RFC 2119 are:

- **MUST** / **REQUIRED** / **SHALL**: Indicates an absolute requirement
- **MUST NOT** / **SHALL NOT**: Indicates an absolute prohibition  
- **SHOULD** / **RECOMMENDED**: Indicates that there may exist valid reasons in particular circumstances to ignore this item, but the full implications must be understood and carefully weighed before choosing a different course
- **SHOULD NOT** / **NOT RECOMMENDED**: Indicates that there may exist valid reasons in particular circumstances when the particular behavior is acceptable or even useful, but the full implications should be understood and the case carefully weighed before implementing
- **MAY** / **OPTIONAL**: Indicates that an item is truly optional

## Consequences

### Positive

- Clear communication of requirement levels reduces ambiguity
- Developers can quickly understand what is mandatory vs. recommended vs. optional
- Consistency with industry standards and other technical specifications
- Better compliance and code review processes

### Negative

- Requires discipline from developers to use the correct terminology
- May seem overly formal for some internal documentation
- Requires education of team members who are unfamiliar with RFC 2119

## Implementation

1. All existing documentation SHOULD be reviewed and updated to use RFC 2119 language where appropriate
2. Code review processes MUST verify that new documentation follows this standard
3. When writing requirements or specifications, authors MUST choose the appropriate RFC 2119 key word based on the actual requirement level
4. This ADR itself serves as an example of proper RFC 2119 usage

## References

- [RFC 2119: Key words for use in RFCs to Indicate Requirement Levels](https://www.rfc-editor.org/rfc/rfc2119.html)
