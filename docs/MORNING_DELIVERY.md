# Morning Delivery

The program can send the daily top-3 report by email or as iPhone push
notifications.

Local configuration lives in `.env`. It is ignored by git because it can contain
secrets. `.env.example` is the template.

## Email

Set these values in `.env` or export them in the shell:

```bash
export BETTING_SMTP_HOST="smtp.example.com"
export BETTING_SMTP_PORT="587"
export BETTING_SMTP_USERNAME="your-smtp-user"
export BETTING_SMTP_PASSWORD="your-smtp-password"
export BETTING_EMAIL_FROM="Betting Agent <agent@example.com>"
export BETTING_EMAIL_TO="you@example.com"
```

Run:

```bash
cargo run -- --norsk-tipping-live \
  --date 2026-05-15 \
  --research examples/research_sources.txt \
  --send-email
```

## iPhone Push

The implemented iPhone option uses Pushover because it has a simple, stable API
and an iOS app. Create a Pushover application, install the iPhone app, then set
these values in `.env`:

```bash
export BETTING_PUSHOVER_TOKEN="your-application-token"
export BETTING_PUSHOVER_USER="your-user-key"
```

Run:

```bash
cargo run -- --norsk-tipping-live \
  --date 2026-05-15 \
  --research examples/research_sources.txt \
  --send-pushover
```

## Cron

Edit `.env`, then add a crontab entry. `BETTING_DELIVERY` can be `email`,
`pushover`, `both`, or `none`. This example runs every morning at 08:00:

```cron
0 8 * * * /home/morten/Prog/betting/scripts/daily_betting.sh >> /home/morten/Prog/betting/daily.log 2>&1
```

The script uses the current local date by default. If the date filter or strict
rules would leave the report empty, the delivered report still includes the top
3 best available candidates with confidence scores and strict-rule warnings.
`BETTING_REFERENCE_ODDS_CSV` is optional and only needed when you want extra
comparison context.

For reliable daily delivery, run the cron job on an always-on machine. See
`docs/DEPLOYMENT.md`.

Sources checked while adding this:

- Pushover Message API: https://pushover.net/api
- Lettre SMTP crate: https://docs.rs/lettre
- POSIX crontab reference: https://man7.org/linux/man-pages/man1/crontab.1p.html
