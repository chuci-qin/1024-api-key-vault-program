# Contributing to 1024 API Key Vault Program

Thank you for your interest in contributing to the 1024 API Key Vault Program! We welcome contributions from the community.

## 📋 Development Setup

### Prerequisites

- Rust 1.75+ (stable channel)
- Solana CLI 1.18+
- Git

### Clone and Build

```bash
git clone https://github.com/1024-org/1024-api-key-vault-program.git
cd 1024-api-key-vault-program
cargo build-sbf
```

### Run Tests

```bash
cargo test
cargo test-sbf
```

## 🔧 Code Style

### General Guidelines

1. **Follow existing patterns**: Study `1024-settlement-program` and `1024-trading-program` for code style reference
2. **Use descriptive names**: Prefer clarity over brevity
3. **Add comments**: Document complex logic, especially in Chinese for team clarity
4. **Error handling**: Always use proper error types, never `unwrap()` in production code
5. **Testing**: Add tests for new features

### Rust Style

- Use `rustfmt` for formatting: `cargo fmt`
- Use `clippy` for linting: `cargo clippy`
- Follow Rust naming conventions:
  - `snake_case` for functions and variables
  - `PascalCase` for types and structs
  - `SCREAMING_SNAKE_CASE` for constants

### Solana-Specific

- Always verify PDA derivation
- Check account ownership before operations
- Use checked arithmetic (`.checked_add()`, `.checked_sub()`)
- Properly handle rent exemption
- Document account requirements in instruction comments

## 📝 Commit Messages

Follow Conventional Commits format:

```
feat: Add delegate expiry validation
fix: Correct PnL calculation in unlock margin
refactor: Simplify token transfer logic
docs: Update API reference
test: Add delegate permission tests
chore: Update dependencies
```

### Commit Message Guidelines

- **Language**: English only
- **Format**: `<type>: <description>`
- **Types**: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`
- **Description**: Start with imperative verb, no period at end
- **Body** (optional): Provide context and reasoning

Example:

```
feat: Add delegate notional limit enforcement

- Implement max_notional check in LockMargin
- Update used_notional on lock/unlock
- Add error VaultError::NotionalLimitExceeded
- Add tests for limit enforcement

Closes #42
```

## 🧪 Testing

### Required Tests for New Features

1. **Happy path**: Feature works as expected
2. **Error cases**: Proper error handling
3. **Edge cases**: Boundary conditions
4. **Security**: Unauthorized access prevented

### Test Structure

```rust
#[tokio::test]
async fn test_feature_name() {
    // Setup
    let program_test = ProgramTest::new(...);
    
    // Execute
    let result = execute_instruction(...);
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(expected, actual);
}
```

## 🔐 Security Considerations

### Critical Checks

1. **Signature verification**: Always verify signer is authorized
2. **Account ownership**: Verify account owner is correct program
3. **PDA derivation**: Always verify PDA matches expected seeds
4. **Arithmetic**: Use checked arithmetic to prevent overflow/underflow
5. **Authorization**: Check permissions before sensitive operations

### Security Review Checklist

- [ ] All signers verified
- [ ] All account owners checked
- [ ] All PDAs verified
- [ ] No unchecked arithmetic
- [ ] No hardcoded addresses
- [ ] Proper error handling
- [ ] No reentrancy vulnerabilities
- [ ] Tests cover security scenarios

## 📤 Pull Request Process

### Before Submitting

1. **Create an issue** describing the change (for non-trivial changes)
2. **Fork the repository** and create a feature branch
3. **Write code** following style guidelines
4. **Add tests** for new functionality
5. **Run tests**: `cargo test-sbf`
6. **Run linter**: `cargo clippy -- -D warnings`
7. **Format code**: `cargo fmt`
8. **Update docs** if adding new features
9. **Update DEVELOPMENT_PROGRESS.md** if applicable

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests pass

## Checklist
- [ ] Code follows project style
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No new warnings
```

### Review Process

1. **Automated checks**: CI must pass
2. **Code review**: At least one maintainer approval required
3. **Security review**: For changes to core logic
4. **Testing**: Reviewer may request additional tests

## 🐛 Bug Reports

### Template

```markdown
**Describe the bug**
Clear description of the bug

**To Reproduce**
Steps to reproduce:
1. Initialize vault with...
2. Call instruction...
3. See error...

**Expected behavior**
What should happen

**Actual behavior**
What actually happens

**Environment**
- Solana version: 
- Rust version:
- OS:

**Additional context**
Any other relevant information
```

## 💡 Feature Requests

### Template

```markdown
**Problem**
What problem does this solve?

**Proposed Solution**
How should it work?

**Alternatives Considered**
Other approaches you thought about

**Additional Context**
Any other relevant information
```

## 📞 Communication

- **GitHub Issues**: Bug reports and feature requests
- **Discord**: General discussion
- **Email**: security@1024.com for security issues

## 📜 License

By contributing, you agree that your contributions will be licensed under the MIT License.

## 🙏 Thank You!

Your contributions make 1024 better for everyone. Thank you for taking the time to contribute!

