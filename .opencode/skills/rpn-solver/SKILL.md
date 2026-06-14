---
name: rpn-solver
description: |-
  Translate standard infix mathematical formulas into Reverse Polish Notation (RPN), 
  and evaluate existing RPN expressions using the project's internal evaluator library.
  Use proactively when a user provides infix formulas (e.g., "3 + 4 * 2") or RPN tokens.
  Examples:
  - user: "convert (5 + 3) * 2 to RPN"
  - user: "solve this RPN string: 5 3 + 2 *"

  The evaluator is at ./uipc-expr.

  You can use the expr-calculator utility to verify formula, e.g.: `cargo run --bin expr-calculator -- 1 1 +`
license: MIT
compatibility: opencode
---

# RPN Solver, Calculator & Converter

## What I do
- Convert standard mathematical expressions (infix) into Reverse Polish Notation (postfix).
- Execute RPN string math calculations correctly.
- Interface with the project's dedicated mathematical evaluation utilities.

## Rules and Guidelines
- **Codebase First**: Do not generate raw standalone Javascript/Python math routines. You must map mathematical evaluations to the codebase's existing evaluator utility located at `uipc-expr`
- **Format Requirements**: Always output the resulting RPN expression cleanly separated by spaces (e.g., `3 4 2 * +`).
- **Step Visualisation**: Show the step-by-step token tracking or stack trace state (Push/Pop operations) when evaluating an RPN expression so the user can verify logic transparency.
