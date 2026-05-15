# GitHub Pages + iPhone Shortcut Setup

This is the recommended low-secret setup:

1. GitHub Actions runs the betting agent every morning.
2. The workflow publishes a static text report to GitHub Pages.
3. Your iPhone Shortcut fetches the report URL and shows a local notification.

No VPS, DNS, Gmail app password, Pushover token, or third-party push app is
required.

## Why This Is The Best Fit

- GitHub hosts the scheduled runner.
- GitHub Pages gives you HTTPS on a `github.io` URL, so no DNS is needed.
- The only secret is a random URL path token.
- Your iPhone keeps notification control locally through Apple Shortcuts.

Tradeoff: GitHub Pages is a static public website. The report is protected by a
long unguessable path, not by login. That is acceptable for low-sensitivity
betting recommendations, but do not put account credentials, stake sizes, or
personal data in the report.

## GitHub Setup

1. Push this repo to GitHub.
2. If the Actions tab says there are no workflows, go to
   `Settings -> Actions -> General` and allow GitHub Actions for this
   repository. The workflow file lives at `.github/workflows/daily-report.yml`
   on the default branch.
3. In the repository, go to `Settings -> Pages`.
4. Set Pages source to `GitHub Actions`.
5. Create a repository secret:

```text
Name: BETTING_REPORT_TOKEN
Value: a long random string, for example 40+ random characters
```

Generate a token locally with:

```bash
openssl rand -hex 32
```

6. Run the workflow once manually:

```text
Actions -> Daily Betting Report -> Run workflow
```

After it deploys, your report URL will be:

```text
https://<github-user>.github.io/<repo-name>/<BETTING_REPORT_TOKEN>/today.txt
```

No custom domain is required.

## iPhone Shortcut

Create a personal automation in Shortcuts:

1. Automation: time of day, for example `08:00`.
2. Action: `Get Contents of URL`.
3. URL:

```text
https://<github-user>.github.io/<repo-name>/<BETTING_REPORT_TOKEN>/today.txt
```

4. Action: `Show Notification`.
5. Notification body: the result from `Get Contents of URL`.
6. Disable `Ask Before Running` if iOS offers that option.

If the notification text is too long, use `Show Result` or `Quick Look` instead
of `Show Notification`, or change the Rust report to produce a shorter iPhone
summary.

## Local Test

Before pushing, test the static publisher locally:

```bash
BETTING_REPORT_TOKEN=test-token BETTING_PUBLIC_DIR=/tmp/betting-public scripts/publish_static_report.sh
cat /tmp/betting-public/test-token/today.txt
```

## Sources

- GitHub Pages can host on `github.io` without a custom domain:
  https://docs.github.com/articles/what-is-github-pages
- GitHub Pages can publish through GitHub Actions:
  https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
- GitHub Actions scheduled workflows use cron:
  https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions
- Apple Shortcuts supports internet content and notifications:
  https://support.apple.com/en-gb/guide/shortcuts/welcome/ios
