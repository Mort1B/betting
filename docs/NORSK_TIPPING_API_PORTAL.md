# Norsk Tipping API Portal Discovery

Discovery date: 2026-05-19

This note records whether `https://api-portal.norsk-tipping.no/` can replace or
enrich the current live Oddsen loader.

## Public Checks

| URL | Public result | Meaning for this repo |
| --- | --- | --- |
| `https://api-portal.norsk-tipping.no/` | HTTP 200 Stoplight workspace shell | The portal exists, but no public sportsbook API documentation rendered in an anonymous browser session. |
| `https://api-portal.norsk-tipping.no/auth` | HTTP 200 Stoplight sign-in page for the Norsk Tipping workspace, with a Norsk Tipping Azure AD option | Access appears to require workspace authentication. |
| `https://api-portal.norsk-tipping.no/openapi.json` | HTTP 404 header and no OpenAPI JSON payload | No anonymous stable OpenAPI export was found at the conventional path. |
| `https://api-portal.norsk-tipping.no/api/v1` | HTTP 404 JSON `Not found` | No obvious public REST API root was found. |
| `https://api-portal.norsk-tipping.no/graphql` | GraphQL endpoint responds, but anonymous introspection is disabled | Anonymous callers cannot discover a useful schema; only trivial public GraphQL checks were possible. |
| `https://api-portal.norsk-tipping.no/robots.txt` | Basic `User-agent: *` response | No discovery blocker was visible, but this is not API permission or documentation. |

## Decision

Do not wire production betting workflows to the API portal yet.

The scheduled workflow should keep using the existing public Oddsen sportsbook
content endpoint:

```text
https://www.norsk-tipping.no/sport/oddsen/sportsbook/services/content/get
```

That source is already implemented by `src/norsk_tipping/client.rs`, supports
same-day football candidate loading, and is the current source of final Norsk
Tipping prices.

## Revisit Criteria

Reconsider the portal only if one of these becomes available:

- Norsk Tipping grants explicit portal access for this use case.
- The portal exposes a stable public OpenAPI, GraphQL schema, or documented
  sportsbook endpoint.
- Authentication requirements, terms, rate limits, and allowed automation are
  known.
- The endpoint covers event IDs, start times, status, markets, selections,
  current odds, and result or settlement data.
- A comparison test can prove parity or improvement against the current live
  Oddsen loader for the same football slate.

## Next Action

If authenticated access is granted, export the relevant OpenAPI/specification
or copy the documented sportsbook endpoint details into a local discovery note.
Without those artifacts, this repo should not add code against the portal.
