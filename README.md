# atlassian-app-auth

[![crates.io](https://img.shields.io/crates/v/atlassian-app-auth.svg)](https://crates.io/crates/atlassian-app-auth)
[![Documentation](https://docs.rs/atlassian-app-auth/badge.svg)](https://docs.rs/atlassian-app-auth)

This is a small library for authenticating with an Atlassian API (such
as the Jira API) as an Atlassian Connect App.

Note that the query string hash implementation is incomplete; there
are a lot of special cases that are not yet handled.

Relevant documentation:
- https://developer.atlassian.com/cloud/jira/platform/integrating-with-jira-cloud
- https://developer.atlassian.com/cloud/jira/platform/security-for-connect-apps
- https://developer.atlassian.com/cloud/jira/platform/understanding-jwt
