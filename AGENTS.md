# Introduction

Hubdash is a GitHub application for monitoring GitHub repositories, focused on
CI/CD pipeline health and how up-to-date the dependencies are, using either
Renovate or Dependabot.

# Conventions

- This app is structured as an HTMX application so as to avoid complex state
  synchronization problems.
- This app is written in Rust. Follow best practices for Rust development.
  - We use Axum for HTTP support and Maud for HTML templates.
  - Skills are provided to explain how to check code quality.
- This app uses Dagger as a workflow/pipelines tool.
