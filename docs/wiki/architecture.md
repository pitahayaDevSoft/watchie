# Architecture and Decisions (ADRs)

This document records the system architecture and Design Decisions (Architecture Decision Records).

## System Overview

`watchie` is a Rust-based terminal application that queries video database metadata and resolves direct download/streaming sources for movie/series titles.

## ADRs

### ADR 0001: Transition from IMDb Scraping to TMDB API

**Status**: Accepted  
**Date**: 2026-06-10  

#### Context

IMDb implements aggressive anti-bot protections (like Cloudflare, JavaScript-only pages, and shifting HTML structural layouts) which render raw HTTP scraping fragile, slow, and frequently blocked.

#### Decision

We transitioned the metadata querying backend to use **TMDB (The Movie Database) API** while preserving the integration with **playimdb.com** (by looking up the original IMDb ID via TMDB's `external_ids` mapping).

#### Consequences

- **Positive**: High speed, reliable JSON results, native support for popular charts/categories, and searches for both movies and TV shows.
- **Negative**: Requires the user to register for a free TMDB API key to run queries.
