# Deployment Recommendation

## Recommended: GitHub Pages + iPhone Shortcut

Use the GitHub Pages workflow in `.github/workflows/daily-report.yml` unless you
specifically need a private authenticated report endpoint.

This avoids:

- renting a VPS,
- DNS,
- storing Gmail credentials on a server,
- Pushover or another third-party push app.

The iPhone fetches the daily report from a long random GitHub Pages path and
shows a local Shortcuts notification. See `docs/GITHUB_PAGES_SHORTCUT.md`.

## Alternative: Always-On Server

For reliable morning delivery, run this on a small always-on machine:

- a cheap VPS,
- a Raspberry Pi or home server,
- or another machine that is awake and online every morning.

Do not rely on a laptop unless it is always powered on, online, and allowed to
run scheduled jobs while sleeping is disabled.

## Minimal Server Setup

1. Install Rust.
2. Clone or copy this repo to `/home/morten/Prog/betting` or adjust
   `BETTING_REPO_DIR` in `.env`.
3. Fill in `.env`.
4. Test locally:

```bash
BETTING_DELIVERY=none scripts/daily_betting.sh
```

5. Test delivery:

```bash
BETTING_DELIVERY=pushover scripts/daily_betting.sh
```

or:

```bash
BETTING_DELIVERY=email scripts/daily_betting.sh
```

6. Add the cron entry:

```cron
0 8 * * * /home/morten/Prog/betting/scripts/daily_betting.sh >> /home/morten/Prog/betting/daily.log 2>&1
```

If the server runs in another timezone, set the cron time accordingly or set the
server timezone to your local timezone.
